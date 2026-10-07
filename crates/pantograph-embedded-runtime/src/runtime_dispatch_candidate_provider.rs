use crate::runtime_dispatch_completion_timing::{
    admitted_task_fingerprint, select_with_owner_timing, EmbeddedCompletionTimingOptIn,
    EmbeddedCompletionTimingQuery,
};
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
    select_scheduler_candidate_for_reservation, SchedulerDispatchCandidate,
    SchedulerDispatchCandidateId, SchedulerDispatchReservationSelection,
    SchedulerDispatchSelectionDiagnostic, SchedulerDispatchSelectionDiagnosticCode,
    SchedulerDispatchSelectionDiagnosticSeverity, SchedulerDispatchSelectionRequest,
    SchedulerEstimateHintKind, SchedulerResourceFitAssessment, SchedulerResourceFitState,
    SchedulerResourceReservation, SchedulerTaskStateRecord,
    ValidatedSchedulerDispatchSelectionRequest, SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
};
use pantograph_workflow_service::workflow::{
    ValidatedWorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateFact,
    WorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateProvider,
    WorkflowRuntimeDispatchCandidateProviderError, WorkflowRuntimeDispatchCandidateSet,
    WorkflowRuntimeDispatchLoadState, WorkflowRuntimeDispatchReservationCustody,
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
use crate::runtime_dispatch_load_target_facts::{
    RuntimeDispatchLoadTargetFact, RuntimeDispatchLoadTargetFactsDiagnostic,
    RuntimeDispatchLoadTargetFactsDiagnosticCode, RuntimeDispatchLoadTargetFactsOutcome,
    RuntimeDispatchLoadTargetFactsProjection,
};
use crate::runtime_dispatch_resource_facts::{
    RuntimeDispatchPublicationOutcome, RuntimeDispatchResourceFactsDiagnostic,
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
const MISSING_RUNTIME_LOAD_TARGET_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_runtime_load_target_facts";
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

#[derive(Clone, Default)]
pub(crate) struct EmbeddedRuntimeDispatchCandidateProvider {
    source_snapshot: EmbeddedRuntimeDispatchCandidateSource,
    resource_facts_source: Option<RuntimeDispatchResourceFactsSource>,
    completion_timing: Option<EmbeddedCompletionTimingOptIn>,
}

#[derive(Clone)]
enum EmbeddedRuntimeDispatchCandidateSource {
    Snapshot(Box<EmbeddedRuntimeDispatchCandidateSourceSnapshot>),
    Store(EmbeddedRuntimeDispatchSourceFactSnapshotStore),
}

impl Default for EmbeddedRuntimeDispatchCandidateSource {
    fn default() -> Self {
        Self::Snapshot(Box::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmbeddedRuntimeDispatchCandidateDraft {
    candidate_id: SchedulerDispatchCandidateId,
    selected_runtime_id: RuntimeIntentId,
    selected_backend_key: String,
    runtime_family: String,
    resolved_load_target: String,
    runtime_residency_key: String,
    selected_model_ref: PumasModelRef,
    content_fingerprint: Option<String>,
    loaded_runtime_memory_estimate_bytes: Option<u64>,
    runtime_status: RuntimeRegistryStatus,
    runtime_instance_id: Option<String>,
    automatic_device_candidates: Vec<inference::gateway::RuntimeOwnedDeviceCandidate>,
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
            source_snapshot: EmbeddedRuntimeDispatchCandidateSource::Snapshot(Box::new(
                source_snapshot,
            )),
            resource_facts_source: None,
            completion_timing: None,
        }
    }

    #[must_use]
    pub(crate) fn with_source_snapshot_store(
        source_snapshot_store: EmbeddedRuntimeDispatchSourceFactSnapshotStore,
    ) -> Self {
        Self {
            source_snapshot: EmbeddedRuntimeDispatchCandidateSource::Store(source_snapshot_store),
            resource_facts_source: None,
            completion_timing: None,
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

impl EmbeddedRuntimeDispatchCandidateProvider {
    pub(crate) fn with_completion_timing(
        mut self,
        timing: Option<EmbeddedCompletionTimingOptIn>,
    ) -> Self {
        self.completion_timing = timing;
        self
    }
}

impl WorkflowRuntimeDispatchCandidateProvider for EmbeddedRuntimeDispatchCandidateProvider {
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        self.runtime_dispatch_candidates_with_inputs(task, ready_record, readiness_proof, None)
    }

    fn requires_materialized_inputs(&self) -> bool {
        self.completion_timing.is_some()
    }

    fn runtime_dispatch_candidates_with_inputs(
        &self,
        task: &WorkflowSchedulerTask,
        ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
        inputs: Option<&[pantograph_runtime_host_contracts::RuntimeHostExecutionInput]>,
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
                ready_record,
                readiness_proof,
                self.completion_timing.as_ref(),
                inputs,
            );
        }
        Ok(WorkflowRuntimeDispatchCandidateSet::from_diagnostics(
            fail_closed_diagnostics(
                &source_snapshot,
                &readiness_proof.preflight_result.identity_key.model_ref,
            ),
        ))
    }
}

impl EmbeddedRuntimeDispatchCandidateSource {
    fn snapshot_for_dispatch(
        &self,
        model_ref: &PumasModelRef,
    ) -> EmbeddedRuntimeDispatchCandidateSourceSnapshot {
        match self {
            Self::Snapshot(snapshot) => snapshot.as_ref().clone(),
            Self::Store(store) => store.snapshot_for_dispatch(model_ref, current_time_ms()),
        }
    }
}

pub(crate) fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn resource_backed_candidate_set(
    source_snapshot: &EmbeddedRuntimeDispatchCandidateSourceSnapshot,
    resource_facts_source: &RuntimeDispatchResourceFactsSource,
    task: &WorkflowSchedulerTask,
    ready_record: &SchedulerTaskStateRecord,
    readiness_proof: &DependencyReadinessProofEnvelope,
    completion_timing: Option<&EmbeddedCompletionTimingOptIn>,
    inputs: Option<&[pantograph_runtime_host_contracts::RuntimeHostExecutionInput]>,
) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError> {
    let model_ref = &readiness_proof.preflight_result.identity_key.model_ref;
    if model_ref.selected_artifact_path.is_some() {
        return Ok(WorkflowRuntimeDispatchCandidateSet::from_diagnostics(vec![
            provider_diagnostic(
                SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                "runtime dispatch candidate provider rejected a path-carrying Pumas model ref",
                PATH_CARRYING_MODEL_REF_HINT,
            ),
        ]));
    }

    let mut diagnostics = Vec::new();
    diagnostics.extend(pumas_package_diagnostics(
        source_snapshot.pumas_package_facts.as_ref(),
    ));
    diagnostics.extend(runtime_capability_diagnostics(
        source_snapshot.runtime_capability_facts.as_ref(),
    ));
    diagnostics.extend(load_target_diagnostics(
        source_snapshot.pumas_load_target_facts.as_ref(),
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
        return Ok(WorkflowRuntimeDispatchCandidateSet::from_diagnostics(
            diagnostics,
        ));
    };
    let Some(environment_ref) = readiness_proof.preflight_result.environment_ref.clone() else {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
            "runtime dispatch candidate provider requires a dependency environment ref",
            MISSING_DEPENDENCY_ENVIRONMENT_REF_HINT,
        ));
        return Ok(WorkflowRuntimeDispatchCandidateSet::from_diagnostics(
            diagnostics,
        ));
    };
    let mut bound_drafts = Vec::new();
    for draft in candidate_drafts {
        // These devices were joined to this registered runtime by its owner
        // backend key. Package hints (e.g. transformers) are library labels,
        // not backend ownership aliases.
        let owned_devices = draft.automatic_device_candidates.iter();
        let bindings: Vec<(
            pantograph_dependency_planning::DeviceIntentId,
            Option<pantograph_scheduler::SchedulerRuntimeVariantId>,
        )> = if let Some(requested) = &task_intent.constraints.requested_device_id {
            // Explicit devices keep their existing evidence/admission path.
            // Discovering CPU never substitutes it for a hard device constraint.
            let matching: Vec<_> = owned_devices
                .filter(|device| device.device_id.as_str() == requested.as_str())
                .map(|device| {
                    (
                        requested.clone(),
                        Some(
                            device
                                .runtime_variant_id
                                .as_str()
                                .parse()
                                .expect("canonical runtime variant"),
                        ),
                    )
                })
                .collect();
            if matching.is_empty() {
                vec![(requested.clone(), None)]
            } else {
                matching
            }
        } else {
            owned_devices
                .map(|device| {
                    (
                        device
                            .device_id
                            .as_str()
                            .parse()
                            .expect("canonical device intent"),
                        Some(
                            device
                                .runtime_variant_id
                                .as_str()
                                .parse()
                                .expect("canonical runtime variant"),
                        ),
                    )
                })
                .collect()
        };
        bound_drafts.extend(
            bindings
                .into_iter()
                .map(|(device, variant)| (draft.clone(), device, variant)),
        );
    }
    if bound_drafts.is_empty() && task_intent.constraints.requested_device_id.is_none() {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::IncompatibleDeviceRequirement,
            "runtime owner has no canonical device candidate for this unconstrained request",
            MISSING_SELECTED_DEVICE_FACTS_HINT,
        ));
    }

    let mut offers = Vec::new();
    let mut evaluated_drafts = Vec::new();
    for (mut draft, selected_device_id, selected_runtime_variant_id) in bound_drafts {
        if let Err(evidence_diagnostic) =
            pre_reservation_evidence_check(&draft, task_intent, selected_device_id.clone())
        {
            diagnostics.push(runtime_dispatch_evidence_diagnostic(
                &draft.candidate_id,
                evidence_diagnostic,
            ));
            continue;
        }
        match resource_facts_source.evaluate(&resource_facts_request(
            &draft,
            task_intent,
            selected_device_id.clone(),
        )) {
            Ok(observation) => {
                draft.runtime_status = observation.runtime_status;
                draft.runtime_instance_id = observation.runtime_instance_id.clone();
                if let Err(evidence_diagnostic) =
                    pre_reservation_evidence_check(&draft, task_intent, selected_device_id.clone())
                {
                    diagnostics.push(runtime_dispatch_evidence_diagnostic(
                        &draft.candidate_id,
                        evidence_diagnostic,
                    ));
                    continue;
                }
                offers.push(SchedulerDispatchCandidate {
                    candidate_id: draft.candidate_id.clone(),
                    selected_runtime_id: draft.selected_runtime_id.clone(),
                    selected_runtime_variant_id: selected_runtime_variant_id.clone(),
                    selected_device_ids: vec![selected_device_id.clone()],
                    selected_model_ref: draft.selected_model_ref.clone(),
                    runtime_trait_settings: task_intent.trait_settings.clone(),
                    reservations: Vec::new(),
                    resource_fit_assessment: Some(fits_assessment(task_intent)),
                    batching_group_id: None,
                    candidate_source_diagnostics: Vec::new(),
                });
                evaluated_drafts.push((
                    draft,
                    observation,
                    selected_device_id,
                    selected_runtime_variant_id,
                ));
            }
            Err(resource_diagnostics) => diagnostics.extend(resource_source_diagnostics(
                &draft.candidate_id,
                &resource_diagnostics,
            )),
        }
    }
    let request =
        ValidatedSchedulerDispatchSelectionRequest::try_from(SchedulerDispatchSelectionRequest {
            contract_version: SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
            task_intent: task_intent.clone(),
            readiness_proof: readiness_proof.clone(),
            environment_ref: environment_ref.clone(),
            candidates: offers,
            diagnostics,
        })
        .map_err(
            |error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: format!("runtime candidate observations failed validation: {error}"),
            },
        )?;
    let selection = if let Some(opt_in) = completion_timing {
        let within_bounds =
            request.as_ref().candidates.len() <= 64 && request.as_ref().diagnostics.len() <= 32;
        let task_fingerprint = within_bounds
            .then(|| admitted_task_fingerprint(task, ready_record))
            .flatten();
        let inputs = if within_bounds {
            inputs.filter(|i| crate::runtime_dispatch_completion_timing::bounded_inputs(i))
                .map(|i| std::sync::Arc::<[pantograph_runtime_host_contracts::RuntimeHostExecutionInput]>::from(i.to_vec()))
        } else {
            None
        };
        let queries: Vec<_> = evaluated_drafts
            .iter()
            .take(if within_bounds { 64 } else { 0 })
            .map(|(draft, observation, device, variant)| {
                let text_bounded = |s: &str| {
                    s.len() <= 128 && !s.trim().is_empty() && !s.chars().any(char::is_control)
                };
                if observation.resources.len() > 8
                    || observation.resource_domains.len() > 8
                    || !observation
                        .resource_domains
                        .iter()
                        .all(|d| text_bounded(&d.domain_id))
                    || !observation
                        .runtime_instance_id
                        .as_deref()
                        .is_none_or(text_bounded)
                    || !text_bounded(&draft.runtime_residency_key)
                    || !draft
                        .content_fingerprint
                        .as_deref()
                        .is_some_and(text_bounded)
                    || task_intent.trait_settings.len() > 32
                    || !task_intent.trait_settings.iter().all(|t| match &t.value {
                        pantograph_scheduler::SchedulerTraitValue::String(v) => v.len() <= 1024,
                        _ => true,
                    })
                    || !task.runtime_source_context.as_ref().is_none_or(|c| {
                        [
                            &c.operation_type,
                            &c.context_shape_key,
                            &c.cancellation_mode,
                        ]
                        .into_iter()
                        .all(|s| text_bounded(s))
                    })
                {
                    return None;
                }
                Some(EmbeddedCompletionTimingQuery {
                    admitted_task_fingerprint: task_fingerprint.clone()?,
                    artifact_fingerprint: draft
                        .content_fingerprint
                        .clone()
                        .filter(|f| !f.trim().is_empty())?,
                    runtime_id: draft.selected_runtime_id.as_str().into(),
                    runtime_variant_id: variant.as_ref().map(|v| v.as_str().to_string()),
                    backend_key: draft.selected_backend_key.clone(),
                    task_kind: task_intent.task_type.as_str().into(),
                    runtime_trait_settings: task_intent.trait_settings.clone(),
                    runtime_source_context: task.runtime_source_context.clone(),
                    materialized_inputs: inputs.as_ref()?.clone(),
                    device_id: device.as_str().into(),
                    runtime_residency_key: draft.runtime_residency_key.clone(),
                    resource_observation: observation.clone(),
                })
            })
            .collect();
        select_with_owner_timing(&request, &queries, opt_in)
    } else {
        select_scheduler_candidate_for_reservation(&request)
    };
    let selected_id = match selection {
        SchedulerDispatchReservationSelection::NoSelection { diagnostics } => {
            return Ok(WorkflowRuntimeDispatchCandidateSet::from_diagnostics(
                diagnostics,
            ));
        }
        SchedulerDispatchReservationSelection::Selected {
            candidate_id,
            diagnostics: selected_diagnostics,
        } => {
            diagnostics = selected_diagnostics;
            candidate_id
        }
    };
    let Some((draft, expected, selected_device_id, selected_runtime_variant_id)) = evaluated_drafts
        .into_iter()
        .find(|(draft, _, _, _)| draft.candidate_id == selected_id)
    else {
        return Err(WorkflowRuntimeDispatchCandidateProviderError::Failed {
            message: "selected candidate has no evaluated source draft".into(),
        });
    };
    let publication = resource_facts_source.reserve_provisional(
        resource_facts_request(&draft, task_intent, selected_device_id.clone()),
        &expected,
        |resource_facts| {
            let evidence_record = runtime_dispatch_evidence_record(
                &draft, selected_device_id.clone(), resource_facts.reservations,
                resource_facts.fit_assessment,
            ).map_err(|error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: format!("runtime dispatch evidence failed before reservation publication: {error:?}"),
            })?;
            let fact = WorkflowRuntimeDispatchCandidateFact {
                candidate_id: draft.candidate_id.clone(),
                selected_runtime_id: evidence_record.selected_runtime_id,
                selected_runtime_variant_id: selected_runtime_variant_id.clone(),
                selected_backend_key: evidence_record.selected_backend_key,
                runtime_family: evidence_record.runtime_family,
                resolved_load_target: evidence_record.resolved_load_target,
                runtime_residency_key: evidence_record.runtime_residency_key,
                loaded_runtime_memory_estimate_bytes: evidence_record.loaded_runtime_memory_estimate_bytes,
                runtime_load_state: workflow_runtime_dispatch_load_state(evidence_record.runtime_load_state),
                runtime_instance_id: evidence_record.runtime_instance_id,
                selected_device_ids: vec![evidence_record.selected_device_id],
                selected_model_ref: evidence_record.selected_model_ref,
                runtime_trait_settings: task_intent.trait_settings.clone(),
                environment_ref: environment_ref.clone(),
                reservations: evidence_record.reservations,
                resource_fit_assessment: evidence_record.resource_fit_assessment,
                batching_group_id: None,
            };
            ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(WorkflowRuntimeDispatchCandidateFactBundle {
                contract_version: pantograph_workflow_service::workflow::WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
                facts: vec![fact], diagnostics: diagnostics.clone(),
            }).map_err(|error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: format!("runtime dispatch candidate facts failed before reservation publication: {error}"),
            })
        },
    )?;
    match publication {
        RuntimeDispatchPublicationOutcome::Reserved { value, custody } => Ok(
            WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(value)
                .with_reservation_custody(Box::new(EmbeddedRuntimeDispatchReservationCustody(
                    custody,
                ))),
        ),
        RuntimeDispatchPublicationOutcome::Unavailable {
            diagnostics: resource_diagnostics,
        } => {
            diagnostics.extend(resource_source_diagnostics(
                &draft.candidate_id,
                &resource_diagnostics,
            ));
            Ok(WorkflowRuntimeDispatchCandidateSet::from_diagnostics(
                diagnostics,
            ))
        }
    }
}

#[derive(Debug)]
struct EmbeddedRuntimeDispatchReservationCustody(
    pantograph_runtime_registry::RuntimeReservationCustody,
);

impl WorkflowRuntimeDispatchReservationCustody for EmbeddedRuntimeDispatchReservationCustody {
    fn transfer(self: Box<Self>) -> Result<(), WorkflowRuntimeDispatchCandidateProviderError> {
        self.0.transfer().map_err(
            |error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: format!("runtime registry custody transfer failed: {error}"),
            },
        )
    }
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
    diagnostics.extend(load_target_diagnostics(
        source_snapshot.pumas_load_target_facts.as_ref(),
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

fn load_target_diagnostics(
    outcome: Option<&RuntimeDispatchLoadTargetFactsOutcome>,
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    match outcome {
        Some(RuntimeDispatchLoadTargetFactsOutcome::Projected { diagnostics, .. }) => diagnostics
            .iter()
            .map(load_target_source_diagnostic)
            .collect(),
        Some(RuntimeDispatchLoadTargetFactsOutcome::Unavailable { diagnostics }) => diagnostics
            .iter()
            .map(load_target_source_diagnostic)
            .collect(),
        None => vec![provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
            "runtime dispatch candidate provider has no staged Pumas load-target facts",
            MISSING_RUNTIME_LOAD_TARGET_FACTS_HINT,
        )],
    }
}

fn load_target_source_diagnostic(
    diagnostic: &RuntimeDispatchLoadTargetFactsDiagnostic,
) -> SchedulerDispatchSelectionDiagnostic {
    provider_diagnostic(
        load_target_diagnostic_code(diagnostic.code),
        &diagnostic.message,
        load_target_diagnostic_hint(diagnostic.code),
    )
}

fn load_target_diagnostic_code(
    code: RuntimeDispatchLoadTargetFactsDiagnosticCode,
) -> SchedulerDispatchSelectionDiagnosticCode {
    match code {
        RuntimeDispatchLoadTargetFactsDiagnosticCode::ReadyResponseMissingTarget
        | RuntimeDispatchLoadTargetFactsDiagnosticCode::SelectedIdentityMismatch
        | RuntimeDispatchLoadTargetFactsDiagnosticCode::EmptyLoadTargetPath => {
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::MissingSelectorAccess
        | RuntimeDispatchLoadTargetFactsDiagnosticCode::UnsupportedSelectorAccessRole
        | RuntimeDispatchLoadTargetFactsDiagnosticCode::MissingRuntimeFamily
        | RuntimeDispatchLoadTargetFactsDiagnosticCode::LoadTargetLookupFailed
        | RuntimeDispatchLoadTargetFactsDiagnosticCode::LoadTargetUnavailable
        | RuntimeDispatchLoadTargetFactsDiagnosticCode::PathFactsStripped => {
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates
        }
    }
}

fn load_target_diagnostic_hint(code: RuntimeDispatchLoadTargetFactsDiagnosticCode) -> &'static str {
    match code {
        RuntimeDispatchLoadTargetFactsDiagnosticCode::MissingSelectorAccess => {
            "embedded_runtime_dispatch_candidate_provider.load_target.missing_selector_access"
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::UnsupportedSelectorAccessRole => {
            "embedded_runtime_dispatch_candidate_provider.load_target.unsupported_selector_access_role"
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::MissingRuntimeFamily => {
            "embedded_runtime_dispatch_candidate_provider.load_target.missing_runtime_family"
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::LoadTargetLookupFailed => {
            "embedded_runtime_dispatch_candidate_provider.load_target.lookup_failed"
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::LoadTargetUnavailable => {
            "embedded_runtime_dispatch_candidate_provider.load_target.unavailable"
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::ReadyResponseMissingTarget => {
            "embedded_runtime_dispatch_candidate_provider.load_target.ready_response_missing_target"
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::EmptyLoadTargetPath => {
            "embedded_runtime_dispatch_candidate_provider.load_target.empty_load_target_path"
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::SelectedIdentityMismatch => {
            "embedded_runtime_dispatch_candidate_provider.load_target.selected_identity_mismatch"
        }
        RuntimeDispatchLoadTargetFactsDiagnosticCode::PathFactsStripped => {
            "embedded_runtime_dispatch_candidate_provider.load_target.path_facts_stripped"
        }
    }
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
    let Some(RuntimeDispatchLoadTargetFactsOutcome::Projected {
        facts: load_target_facts,
        ..
    }) = source_snapshot.pumas_load_target_facts.as_ref()
    else {
        return (Vec::new(), Vec::new());
    };

    candidate_drafts_from_projected_facts(package_facts, capability_facts, load_target_facts)
}

fn candidate_drafts_from_projected_facts(
    package_facts: &PumasDispatchPackageFactsProjection,
    capability_facts: &RuntimeDispatchCapabilityFactsProjection,
    load_target_facts: &RuntimeDispatchLoadTargetFactsProjection,
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

    let mut drafts = Vec::new();
    let mut diagnostics = Vec::new();
    for runtime in &capability_facts.runtimes {
        let matching_backend_keys = runtime
            .backend_keys
            .iter()
            .map(|backend_key| normalize_backend_key(backend_key))
            .filter(|backend_key| backend_hint_keys.contains(backend_key))
            .collect::<Vec<_>>();
        if matching_backend_keys.is_empty() {
            continue;
        }
        let Some(load_target) = load_target_for_runtime(load_target_facts, &runtime.runtime_family)
        else {
            diagnostics.push(provider_diagnostic(
                SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                "runtime dispatch candidate provider has no Pumas load-target fact for a backend-compatible runtime family",
                MISSING_RUNTIME_LOAD_TARGET_FACTS_HINT,
            ));
            continue;
        };
        drafts.extend(matching_backend_keys.into_iter().map(|backend_key| {
            runtime_candidate_draft(package_facts, runtime, load_target, backend_key)
        }));
    }

    if drafts.is_empty() {
        return (
            drafts,
            if diagnostics.is_empty() {
                vec![provider_diagnostic(
                    SchedulerDispatchSelectionDiagnosticCode::IncompatibleRuntimeRequirement,
                    "no runtime registry capability facts match the Pumas package backend hints",
                    INCOMPATIBLE_RUNTIME_BACKEND_HINT,
                )]
            } else {
                diagnostics
            },
        );
    }

    (drafts, diagnostics)
}

fn load_target_for_runtime<'a>(
    load_target_facts: &'a RuntimeDispatchLoadTargetFactsProjection,
    runtime_family: &str,
) -> Option<&'a RuntimeDispatchLoadTargetFact> {
    load_target_facts
        .load_targets
        .iter()
        .find(|load_target| load_target.runtime_family == runtime_family)
}

fn runtime_candidate_draft(
    package_facts: &PumasDispatchPackageFactsProjection,
    runtime: &RuntimeDispatchRuntimeCapabilityFacts,
    load_target: &RuntimeDispatchLoadTargetFact,
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
        runtime_family: runtime.runtime_family.clone(),
        resolved_load_target: load_target.resolved_load_target.clone(),
        runtime_residency_key: runtime.runtime_residency_key.clone(),
        selected_model_ref: load_target.model_ref.clone(),
        content_fingerprint: load_target.content_fingerprint.clone(),
        loaded_runtime_memory_estimate_bytes: conservative_loaded_runtime_memory_estimate_bytes(
            &package_facts.logical_size,
        ),
        runtime_status: runtime.status,
        runtime_instance_id: runtime.runtime_instance_id.clone(),
        automatic_device_candidates: runtime.automatic_device_candidates.clone(),
    }
}

fn pre_reservation_evidence_check(
    draft: &EmbeddedRuntimeDispatchCandidateDraft,
    task_intent: &pantograph_scheduler::SchedulableTaskIntent,
    selected_device_id: pantograph_dependency_planning::DeviceIntentId,
) -> Result<(), RuntimeDispatchEvidenceDiagnostic> {
    RuntimeDispatchEvidenceRequest {
        selected_backend_key: draft.selected_backend_key.clone(),
        runtime_family: draft.runtime_family.clone(),
        resolved_load_target: draft.resolved_load_target.clone(),
        runtime_residency_key: draft.runtime_residency_key.clone(),
        loaded_runtime_memory_estimate_bytes: draft
            .loaded_runtime_memory_estimate_bytes
            .unwrap_or_default(),
        runtime_load_state: Some(runtime_dispatch_evidence_load_state(draft.runtime_status)),
        runtime_instance_id: draft.runtime_instance_id.clone(),
        selected_runtime_id: draft.selected_runtime_id.clone(),
        selected_model_ref: draft.selected_model_ref.clone(),
        selected_device_id,
        reservations: Vec::new(),
        resource_fit_assessment: fits_assessment(task_intent),
    }
    .validate_candidate_identity()
}

fn runtime_dispatch_evidence_record(
    draft: &EmbeddedRuntimeDispatchCandidateDraft,
    selected_device_id: pantograph_dependency_planning::DeviceIntentId,
    reservations: Vec<SchedulerResourceReservation>,
    resource_fit_assessment: SchedulerResourceFitAssessment,
) -> Result<RuntimeDispatchEvidenceRecord, RuntimeDispatchEvidenceDiagnostic> {
    RuntimeDispatchEvidenceRecord::new(RuntimeDispatchEvidenceRequest {
        selected_backend_key: draft.selected_backend_key.clone(),
        runtime_family: draft.runtime_family.clone(),
        resolved_load_target: draft.resolved_load_target.clone(),
        runtime_residency_key: draft.runtime_residency_key.clone(),
        loaded_runtime_memory_estimate_bytes: draft
            .loaded_runtime_memory_estimate_bytes
            .unwrap_or_default(),
        runtime_load_state: Some(runtime_dispatch_evidence_load_state(draft.runtime_status)),
        runtime_instance_id: draft.runtime_instance_id.clone(),
        selected_runtime_id: draft.selected_runtime_id.clone(),
        selected_model_ref: draft.selected_model_ref.clone(),
        selected_device_id,
        reservations,
        resource_fit_assessment,
    })
}

fn fits_assessment(
    task_intent: &pantograph_scheduler::SchedulableTaskIntent,
) -> SchedulerResourceFitAssessment {
    SchedulerResourceFitAssessment {
        workflow_run_id: task_intent.workflow_run_id.clone(),
        task_id: task_intent.task_id.clone(),
        state: SchedulerResourceFitState::Fits,
        diagnostics: Vec::new(),
    }
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

fn workflow_runtime_dispatch_load_state(
    load_state: RuntimeDispatchEvidenceLoadState,
) -> WorkflowRuntimeDispatchLoadState {
    match load_state {
        RuntimeDispatchEvidenceLoadState::NotLoaded => WorkflowRuntimeDispatchLoadState::NotLoaded,
        RuntimeDispatchEvidenceLoadState::Loading => WorkflowRuntimeDispatchLoadState::Loading,
        RuntimeDispatchEvidenceLoadState::Loaded => WorkflowRuntimeDispatchLoadState::Loaded,
        RuntimeDispatchEvidenceLoadState::Busy => WorkflowRuntimeDispatchLoadState::Busy,
        RuntimeDispatchEvidenceLoadState::Unloading => WorkflowRuntimeDispatchLoadState::Unloading,
        RuntimeDispatchEvidenceLoadState::Failed => WorkflowRuntimeDispatchLoadState::Failed,
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
pub(crate) use tests::completion_test_provider;

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
    fn private_source_indirections_preserve_raw_facts_and_snapshot_clone_isolation() {
        let expected = source_snapshot(Vec::new(), Vec::new());
        let expected_facts = pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]);
        let provider =
            EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(expected.clone());
        let mut returned = provider
            .source_snapshot
            .snapshot_for_dispatch(&path_free_model_ref());
        assert_eq!(returned, expected);
        let Some(PumasDispatchPackageFactsBridgeOutcome::Projected { facts, diagnostics }) =
            returned.pumas_package_facts.as_mut()
        else {
            panic!("snapshot must preserve projected package facts");
        };
        assert_eq!(facts.as_ref(), &expected_facts);
        assert!(diagnostics.is_empty());
        facts.model_ref.model_id = "changed-returned-copy".to_string();
        returned.snapshot_version = 99;
        assert_eq!(
            provider
                .source_snapshot
                .snapshot_for_dispatch(&path_free_model_ref()),
            expected
        );
        assert!(std::mem::size_of::<PumasDispatchPackageFactsBridgeOutcome>() <= 64);
        assert!(std::mem::size_of::<EmbeddedRuntimeDispatchCandidateSource>() <= 64);
    }

    #[test]
    fn default_provider_retains_empty_snapshot_and_fail_closed_diagnostics() {
        let provider = EmbeddedRuntimeDispatchCandidateProvider::default();
        assert!(provider.resource_facts_source.is_none());
        let EmbeddedRuntimeDispatchCandidateSource::Snapshot(snapshot) = provider.source_snapshot
        else {
            panic!("default provider must retain snapshot mode");
        };
        assert_eq!(
            *snapshot,
            EmbeddedRuntimeDispatchCandidateSourceSnapshot::default()
        );
        let diagnostics = fail_closed_diagnostics(&snapshot, &path_free_model_ref());
        assert_eq!(diagnostics.len(), 4);
        assert!(diagnostics.iter().all(|diagnostic| diagnostic.code
            == SchedulerDispatchSelectionDiagnosticCode::NoCandidates
            && diagnostic.severity == SchedulerDispatchSelectionDiagnosticSeverity::Error));
    }

    #[test]
    fn fail_closed_provider_reports_missing_source_facts() {
        let diagnostics = fail_closed_diagnostics(
            &EmbeddedRuntimeDispatchCandidateSourceSnapshot::default(),
            &path_free_model_ref(),
        );

        assert_eq!(diagnostics.len(), 4);
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.candidate_id.is_none()));
        assert!(has_hint(&diagnostics, MISSING_PUMAS_PACKAGE_FACTS_HINT));
        assert!(has_hint(
            &diagnostics,
            MISSING_RUNTIME_CAPABILITY_FACTS_HINT
        ));
        assert!(has_hint(
            &diagnostics,
            MISSING_RUNTIME_LOAD_TARGET_FACTS_HINT
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

        assert_eq!(diagnostics.len(), 4);
        assert!(!has_hint(&diagnostics, MISSING_PUMAS_PACKAGE_FACTS_HINT));
        assert!(!has_hint(
            &diagnostics,
            MISSING_RUNTIME_CAPABILITY_FACTS_HINT
        ));
        assert!(has_hint(
            &diagnostics,
            MISSING_RUNTIME_LOAD_TARGET_FACTS_HINT
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
            &load_target_facts(vec![load_target("diffusers")]),
        );

        assert!(diagnostics.is_empty());
        assert_eq!(drafts.len(), 1);
        let draft = &drafts[0];
        assert_eq!(draft.candidate_id.as_str(), "runtime.pytorch");
        assert_eq!(draft.selected_runtime_id.as_str(), "pytorch");
        assert_eq!(draft.selected_backend_key, "diffusers");
        assert_eq!(draft.selected_model_ref.model_id, "pumas.model.sdxl");
        assert_eq!(draft.selected_model_ref.selected_artifact_path, None);
        assert_eq!(draft.runtime_family, "diffusers");
        assert_eq!(
            draft.resolved_load_target,
            "pumas:pumas.model.sdxl:diffusers"
        );
        assert_eq!(
            draft.runtime_residency_key,
            "runtime.diffusers.pytorch.shared"
        );
        assert_eq!(draft.loaded_runtime_memory_estimate_bytes, Some(23_856));
    }

    #[test]
    fn candidate_drafts_report_backend_hint_mismatch() {
        let (drafts, diagnostics) = candidate_drafts_from_projected_facts(
            &pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]),
            &runtime_capability_facts(vec![runtime_capability("llama.cpp", vec!["llama.cpp"])]),
            &load_target_facts(vec![load_target("diffusers")]),
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
            &load_target_facts(vec![load_target("diffusers")]),
        );

        assert!(diagnostics.is_empty());
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].loaded_runtime_memory_estimate_bytes, None);
    }

    #[test]
    fn mismatched_load_target_identity_cannot_reserve_a_candidate() {
        let registry = dispatch_registry();
        let mut snapshot = source_snapshot(
            vec![runtime_capability("pytorch", vec!["diffusers"])],
            Vec::new(),
        );
        snapshot.pumas_load_target_facts =
            Some(RuntimeDispatchLoadTargetFactsOutcome::Unavailable {
                diagnostics: vec![RuntimeDispatchLoadTargetFactsDiagnostic {
                    code: RuntimeDispatchLoadTargetFactsDiagnosticCode::SelectedIdentityMismatch,
                    runtime_family: Some("diffusers".to_string()),
                    message: "selected model identity disagrees with the ready target".to_string(),
                }],
            });
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(snapshot)
            .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));
        let result = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .expect("identity mismatch must remain a typed rejection");
        assert!(result.candidates.is_empty());
        assert!(result.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
                && diagnostic.hint.as_deref()
                    == Some("embedded_runtime_dispatch_candidate_provider.load_target.selected_identity_mismatch")
        }));
        assert!(registry.snapshot().reservations.is_empty());
    }

    #[test]
    fn provider_fails_closed_when_load_target_evidence_is_missing() {
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
                    facts: Box::new(pumas_package_facts(vec![
                        inference::BackendHintLabel::Diffusers,
                    ])),
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
        assert!(has_hint(
            &candidate_set.diagnostics,
            MISSING_RUNTIME_LOAD_TARGET_FACTS_HINT
        ));
        assert_eq!(registry.snapshot().reservations.len(), 0);
    }

    #[test]
    fn provider_fails_closed_when_loaded_runtime_has_no_instance_id() {
        let registry = dispatch_registry();
        let mut runtime = runtime_capability("pytorch", vec!["diffusers"]);
        runtime.runtime_instance_id = None;
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(
            source_snapshot(vec![runtime], vec![load_target("diffusers")]),
        )
        .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));

        let candidate_set = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .expect("missing loaded runtime instance should be a typed diagnostic");

        assert!(candidate_set.candidates.is_empty());
        assert!(candidate_set.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .candidate_id
                .as_ref()
                .is_some_and(|candidate_id| candidate_id.as_str() == "runtime.pytorch")
                && diagnostic
                    .hint
                    .as_deref()
                    .is_some_and(|hint| hint.ends_with(":runtime_instance_id"))
        }));
        assert_eq!(registry.snapshot().reservations.len(), 0);
    }

    #[test]
    fn provider_fails_closed_when_runtime_family_evidence_is_missing() {
        let registry = dispatch_registry();
        let mut runtime = runtime_capability("pytorch", vec!["diffusers"]);
        runtime.runtime_family.clear();
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(
            source_snapshot(vec![runtime], vec![load_target("")]),
        )
        .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));

        let candidate_set = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .expect("missing runtime family should be a typed diagnostic");

        assert!(candidate_set.candidates.is_empty());
        assert!(candidate_set.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .hint
                .as_deref()
                .is_some_and(|hint| hint.ends_with(":runtime_family"))
        }));
        assert_eq!(registry.snapshot().reservations.len(), 0);
    }

    #[test]
    fn provider_fails_closed_when_residency_key_evidence_is_missing() {
        let registry = dispatch_registry();
        let mut runtime = runtime_capability("pytorch", vec!["diffusers"]);
        runtime.runtime_residency_key.clear();
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(
            source_snapshot(vec![runtime], vec![load_target("diffusers")]),
        )
        .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));

        let candidate_set = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .expect("missing residency key should be a typed diagnostic");

        assert!(candidate_set.candidates.is_empty());
        assert!(candidate_set.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .hint
                .as_deref()
                .is_some_and(|hint| hint.ends_with(":runtime_residency_key"))
        }));
        assert_eq!(registry.snapshot().reservations.len(), 0);
    }

    #[test]
    fn provider_emits_candidate_when_dispatch_evidence_is_complete() {
        let registry = dispatch_registry();
        let provider =
            EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(source_snapshot(
                vec![runtime_capability("pytorch", vec!["diffusers"])],
                vec![load_target("diffusers")],
            ))
            .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));

        let candidate_set = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .expect("complete dispatch evidence should emit a candidate");

        assert_eq!(candidate_set.candidates.len(), 1);
        let candidate = &candidate_set.candidates[0];
        assert_eq!(candidate.candidate_id.as_str(), "runtime.pytorch");
        assert_eq!(candidate.selected_runtime_id.as_str(), "pytorch");
        assert_eq!(
            candidate.selected_model_ref.selected_artifact_id.as_deref(),
            Some("diffusers")
        );
        assert_eq!(candidate.selected_model_ref.selected_artifact_path, None);
        assert_eq!(candidate.selected_device_ids.len(), 1);
        assert_eq!(registry.snapshot().reservations.len(), 1);
    }

    #[test]
    fn provider_ambiguous_or_duplicate_alternatives_leave_registry_unreserved() {
        for duplicate in [false, true] {
            let registry = dispatch_registry();
            registry.register_runtime(
                pantograph_runtime_registry::RuntimeRegistration::new("pytorch-alt", "Alternative")
                    .with_backend_keys(vec!["diffusers".into()])
                    .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                        RuntimeAdmissionResourceBudget::ram_bytes(Some(16 * mib())),
                        RuntimeAdmissionResourceBudget::vram_bytes(Some(8 * mib())),
                    ])),
            );
            let first = runtime_capability("pytorch", vec!["diffusers"]);
            let second = if duplicate {
                first.clone()
            } else {
                runtime_capability("pytorch-alt", vec!["diffusers"])
            };
            let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(
                source_snapshot(vec![first, second], vec![load_target("diffusers")]),
            )
            .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));
            let mut task = workflow_task(Some("cuda:0"));
            task.schedulable_intent
                .as_mut()
                .unwrap()
                .constraints
                .requested_runtime_id = None;
            let result = provider
                .runtime_dispatch_candidates(&task, &ready_record(), &readiness_proof())
                .unwrap();
            assert!(result.candidates.is_empty());
            assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code
                == if duplicate {
                    SchedulerDispatchSelectionDiagnosticCode::DuplicateCandidateId
                } else {
                    SchedulerDispatchSelectionDiagnosticCode::AmbiguousRanking
                }));
            assert!(registry.snapshot().reservations.is_empty());
        }
    }

    #[test]
    fn provider_ignores_unrequested_alternative_before_acquiring_exactly_one_lease() {
        let registry = dispatch_registry();
        registry.register_runtime(
            pantograph_runtime_registry::RuntimeRegistration::new("pytorch-alt", "Alternative")
                .with_backend_keys(vec!["diffusers".into()])
                .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                    RuntimeAdmissionResourceBudget::ram_bytes(Some(16 * mib())),
                    RuntimeAdmissionResourceBudget::vram_bytes(Some(8 * mib())),
                ])),
        );
        let provider =
            EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(source_snapshot(
                vec![
                    runtime_capability("pytorch-alt", vec!["diffusers"]),
                    runtime_capability("pytorch", vec!["diffusers"]),
                ],
                vec![load_target("diffusers")],
            ))
            .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));
        let result = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .unwrap();
        assert_eq!(result.candidates.len(), 1);
        let snapshot = registry.snapshot();
        assert_eq!(snapshot.reservations.len(), 1);
        assert_eq!(snapshot.reservations[0].runtime_id, "pytorch");
        assert_eq!(snapshot.reservations[0].reservation_id, 1);
        assert_eq!(
            result.candidates[0].candidate_id.as_str(),
            "runtime.pytorch"
        );
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
                    facts: Box::new(pumas_package_facts(vec![
                        inference::BackendHintLabel::Diffusers,
                    ])),
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

    fn cpu_runtime_capability(runtime_id: &str) -> RuntimeDispatchRuntimeCapabilityFacts {
        let mut runtime = runtime_capability(runtime_id, vec!["diffusers"]);
        runtime
            .automatic_device_candidates
            .push(inference::gateway::RuntimeOwnedDeviceCandidate {
                backend_key: "pytorch".into(),
                runtime_variant_id: "pytorch.cpu".parse().unwrap(),
                device_id: "cpu".parse().unwrap(),
            });
        runtime
    }

    #[test]
    fn unconstrained_cpu_selection_keeps_full_peak_claims_and_rolls_back_untransferred_custody() {
        let registry = dispatch_registry();
        let provider =
            EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(source_snapshot(
                vec![cpu_runtime_capability("pytorch")],
                vec![load_target("diffusers")],
            ))
            .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));
        let result = provider
            .runtime_dispatch_candidates(&workflow_task(None), &ready_record(), &readiness_proof())
            .unwrap();
        assert_eq!(result.candidates.len(), 1, "{:?}", result.diagnostics);
        let selected = &result.candidates[0];
        assert_eq!(selected.selected_device_ids[0].as_str(), "cpu");
        assert_eq!(
            selected
                .selected_runtime_variant_id
                .as_ref()
                .unwrap()
                .as_str(),
            "pytorch.cpu"
        );
        assert_eq!(registry.snapshot().reservations.len(), 1);
        assert_eq!(
            selected
                .reservations
                .iter()
                .map(|claim| claim.reserved_bytes)
                .sum::<u64>(),
            3 * mib()
        );
        drop(result);
        assert!(registry.snapshot().reservations.is_empty());
    }

    #[test]
    fn automatic_cpu_alternatives_and_hard_runtime_constraints_never_acquire_speculative_leases() {
        let registry = dispatch_registry();
        registry.register_runtime(
            pantograph_runtime_registry::RuntimeRegistration::new("pytorch-alt", "alternative")
                .with_backend_keys(vec!["diffusers".into()])
                .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                    RuntimeAdmissionResourceBudget::ram_bytes(Some(16 * mib())),
                    RuntimeAdmissionResourceBudget::vram_bytes(Some(8 * mib())),
                ])),
        );
        let provider =
            EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(source_snapshot(
                vec![
                    cpu_runtime_capability("pytorch"),
                    cpu_runtime_capability("pytorch-alt"),
                ],
                vec![load_target("diffusers")],
            ))
            .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));
        for runtime_constraint in [None, Some("unsupported")] {
            let mut task = workflow_task(None);
            task.schedulable_intent
                .as_mut()
                .unwrap()
                .constraints
                .requested_runtime_id = runtime_constraint.map(|id| id.parse().unwrap());
            let result = provider
                .runtime_dispatch_candidates(&task, &ready_record(), &readiness_proof())
                .unwrap();
            assert!(result.candidates.is_empty());
            assert!(registry.snapshot().reservations.is_empty());
            if runtime_constraint.is_none() {
                assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code
                    == SchedulerDispatchSelectionDiagnosticCode::AmbiguousRanking));
            }
        }
        let result = provider
            .runtime_dispatch_candidates(&workflow_task(None), &ready_record(), &readiness_proof())
            .unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.candidates[0].selected_runtime_id.as_str(), "pytorch");
        assert_eq!(registry.snapshot().reservations[0].reservation_id, 1);
    }

    #[test]
    fn cpu_discovery_preserves_explicit_device_and_authoritative_admission() {
        let registry = dispatch_registry();
        let provider =
            EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(source_snapshot(
                vec![cpu_runtime_capability("pytorch")],
                vec![load_target("diffusers")],
            ))
            .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));
        let result = provider
            .runtime_dispatch_candidates(
                &workflow_task(Some("cuda:0")),
                &ready_record(),
                &readiness_proof(),
            )
            .unwrap();
        assert_eq!(result.candidates.len(), 1);
        assert_eq!(
            result.candidates[0].selected_device_ids[0].as_str(),
            "cuda:0"
        );
        assert!(result.candidates[0].selected_runtime_variant_id.is_none());
        drop(result);
        let mut task = workflow_task(None);
        task.schedulable_intent.as_mut().unwrap().estimate_hints[0].value = 17 * mib();
        let result = provider
            .runtime_dispatch_candidates(&task, &ready_record(), &readiness_proof())
            .unwrap();
        assert!(result.candidates.is_empty());
        assert!(registry.snapshot().reservations.is_empty());
    }

    #[test]
    fn duplicate_owned_cpu_bindings_fail_closed_before_any_lease_for_explicit_or_automatic_choice()
    {
        let registry = dispatch_registry();
        let mut runtime = cpu_runtime_capability("pytorch");
        runtime
            .automatic_device_candidates
            .push(runtime.automatic_device_candidates[0].clone());
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(
            source_snapshot(vec![runtime], vec![load_target("diffusers")]),
        )
        .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));
        for requested in [None, Some("cpu")] {
            let result = provider
                .runtime_dispatch_candidates(
                    &workflow_task(requested),
                    &ready_record(),
                    &readiness_proof(),
                )
                .unwrap();
            assert!(result.candidates.is_empty());
            assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code
                == SchedulerDispatchSelectionDiagnosticCode::DuplicateCandidateId));
            assert!(registry.snapshot().reservations.is_empty());
        }
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
            automatic_device_candidates: Vec::new(),
        }
    }

    fn load_target(runtime_family: &str) -> RuntimeDispatchLoadTargetFact {
        RuntimeDispatchLoadTargetFact {
            runtime_family: runtime_family.to_string(),
            resolved_load_target: "pumas:pumas.model.sdxl:diffusers".to_string(),
            model_ref: path_free_model_ref(),
            artifact_kind: "DiffusersBundle".to_string(),
            load_path_kind: "Directory".to_string(),
            library_root_id: Some("default".to_string()),
            storage_kind: "LibraryOwned".to_string(),
            validation_state: "Valid".to_string(),
            content_fingerprint: Some("sha256:abc".to_string()),
            package_facts_contract_version: Some(1),
        }
    }

    fn load_target_facts(
        load_targets: Vec<RuntimeDispatchLoadTargetFact>,
    ) -> RuntimeDispatchLoadTargetFactsProjection {
        RuntimeDispatchLoadTargetFactsProjection { load_targets }
    }

    fn source_snapshot(
        runtimes: Vec<RuntimeDispatchRuntimeCapabilityFacts>,
        load_targets: Vec<RuntimeDispatchLoadTargetFact>,
    ) -> EmbeddedRuntimeDispatchCandidateSourceSnapshot {
        EmbeddedRuntimeDispatchCandidateSourceSnapshot {
            pumas_package_facts: Some(PumasDispatchPackageFactsBridgeOutcome::Projected {
                facts: Box::new(pumas_package_facts(vec![
                    inference::BackendHintLabel::Diffusers,
                ])),
                diagnostics: Vec::new(),
            }),
            runtime_capability_facts: Some(RuntimeDispatchCapabilityFactsOutcome::Projected {
                facts: runtime_capability_facts(runtimes),
                diagnostics: Vec::new(),
            }),
            pumas_load_target_facts: Some(RuntimeDispatchLoadTargetFactsOutcome::Projected {
                facts: load_target_facts(load_targets),
                diagnostics: Vec::new(),
            }),
            ..EmbeddedRuntimeDispatchCandidateSourceSnapshot::default()
        }
    }

    // Controlled owner facts for the public-session qualification tests only.
    pub(crate) fn completion_test_provider(
        model_ref: PumasModelRef,
        timing: Option<EmbeddedCompletionTimingOptIn>,
    ) -> (
        EmbeddedRuntimeDispatchCandidateProvider,
        Arc<RuntimeRegistry>,
    ) {
        completion_test_provider_with_count(model_ref, timing, 2)
    }

    fn completion_test_provider_with_count(
        model_ref: PumasModelRef,
        timing: Option<EmbeddedCompletionTimingOptIn>,
        count: usize,
    ) -> (
        EmbeddedRuntimeDispatchCandidateProvider,
        Arc<RuntimeRegistry>,
    ) {
        let registry = dispatch_registry();
        let ids: Vec<_> = (0..count)
            .map(|i| match i {
                0 => "pytorch".to_string(),
                1 => "pytorch-alt".to_string(),
                _ => format!("pytorch-alt-{i}"),
            })
            .collect();
        for id in &ids {
            registry.register_runtime(
                pantograph_runtime_registry::RuntimeRegistration::new(id, id)
                    .with_backend_keys(vec!["diffusers".into()])
                    .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                        RuntimeAdmissionResourceBudget::ram_bytes(Some(16 * 1024 * mib())),
                        RuntimeAdmissionResourceBudget::vram_bytes(Some(8 * 1024 * mib())),
                    ])),
            );
        }
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime.pytorch.001".into()),
                },
            )
            .unwrap();
        let mut target = load_target("diffusers");
        target.model_ref = model_ref.clone();
        let mut snapshot = source_snapshot(
            ids.iter()
                .map(|id| {
                    let mut facts = runtime_capability(id, vec!["diffusers"]);
                    facts.automatic_device_candidates.push(
                        inference::gateway::RuntimeOwnedDeviceCandidate {
                            backend_key: "diffusers".into(),
                            runtime_variant_id: inference::RuntimeVariantId::parse("pytorch.cpu")
                                .unwrap(),
                            device_id: inference::InferenceDeviceId::parse("cpu").unwrap(),
                        },
                    );
                    facts
                })
                .collect(),
            vec![target],
        );
        if let Some(PumasDispatchPackageFactsBridgeOutcome::Projected { facts, .. }) =
            snapshot.pumas_package_facts.as_mut()
        {
            facts.model_ref = model_ref;
        }
        (
            EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(snapshot)
                .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(
                    registry.clone(),
                ))
                .with_completion_timing(timing),
            registry,
        )
    }

    #[test]
    fn completion_native_oversized_cohort_never_calls_owner_or_reserves() {
        struct Source;
        impl crate::EmbeddedCompletionTimingSource for Source {
            fn timing_for(
                &self,
                _: &EmbeddedCompletionTimingQuery,
            ) -> Option<crate::EmbeddedCompletionTimingRecord> {
                panic!("oversized cohort must refuse before owner callback");
            }
        }
        let (provider, registry) = completion_test_provider_with_count(
            path_free_model_ref(),
            Some(EmbeddedCompletionTimingOptIn {
                source: Arc::new(Source),
                owner_epoch: "owner".into(),
                max_sample_age_ms: 1000,
                allow_configured_estimates: true,
            }),
            65,
        );
        let mut task = workflow_task(Some("cpu"));
        task.schedulable_intent
            .as_mut()
            .unwrap()
            .constraints
            .requested_runtime_id = None;
        let result = provider
            .runtime_dispatch_candidates_with_inputs(
                &task,
                &ready_record(),
                &readiness_proof(),
                Some(&[]),
            )
            .unwrap();
        assert!(result.candidates.is_empty());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("WorkLimitExceeded")));
        assert!(registry.snapshot().reservations.is_empty());
    }

    #[test]
    #[ignore = "controlled native dispatch cost probe; run explicitly with --ignored --nocapture"]
    fn completion_native_dispatch_cost_probe() {
        use crate::{EmbeddedCompletionTimingRecord, EmbeddedCompletionTimingSource};
        struct Source;
        impl EmbeddedCompletionTimingSource for Source {
            fn timing_for(
                &self,
                q: &EmbeddedCompletionTimingQuery,
            ) -> Option<EmbeddedCompletionTimingRecord> {
                let value = || inference::RuntimeServiceTimingValue::ConfiguredEstimate {
                    elapsed_ns: 1000,
                };
                Some(EmbeddedCompletionTimingRecord {
                    query: q.clone(),
                    observed_at_ms: current_time_ms(),
                    preparation: value(),
                    required_transfer: value(),
                    execution: value(),
                })
            }
        }
        for (count, input_bytes, samples) in [
            (1, 256, 1000),
            (2, 256, 1000),
            (64, 256, 500),
            (64, 60 * 1024, 200),
        ] {
            let (provider, registry) = completion_test_provider_with_count(
                path_free_model_ref(),
                Some(EmbeddedCompletionTimingOptIn {
                    source: Arc::new(Source),
                    owner_epoch: "controlled-cost-probe".into(),
                    max_sample_age_ms: 1000,
                    allow_configured_estimates: true,
                }),
                count,
            );
            let mut task = workflow_task(Some("cpu"));
            task.schedulable_intent
                .as_mut()
                .unwrap()
                .constraints
                .requested_runtime_id = None;
            let inputs = vec![
                pantograph_runtime_host_contracts::RuntimeHostExecutionInput {
                    port_id: "prompt".into(),
                    value:
                        pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String(
                            "x".repeat(input_bytes),
                        ),
                },
            ];
            let ready = ready_record();
            let proof = readiness_proof();
            let mut times = Vec::new();
            for i in 0..samples + 20 {
                let start = std::time::Instant::now();
                let result = provider
                    .runtime_dispatch_candidates_with_inputs(&task, &ready, &proof, Some(&inputs))
                    .unwrap();
                assert_eq!(result.candidates.len(), 1);
                assert_eq!(registry.snapshot().reservations.len(), 1);
                drop(result); // provisional custody rolls back the winner
                assert!(registry.snapshot().reservations.is_empty());
                if i >= 20 {
                    times.push(start.elapsed().as_micros());
                }
            }
            times.sort_unstable();
            println!("native_dispatch candidates={count} input_bytes={input_bytes} samples={samples} median_us={} p95_us={} max_us={}",
                times[times.len()/2], times[times.len()*95/100], times[times.len()-1]);
        }
    }

    fn dispatch_registry() -> Arc<RuntimeRegistry> {
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
        registry
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
            runtime_source_context: None,
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
                execution_intent: pantograph_scheduler::SchedulerTaskExecutionIntent::runtime(
                    schedulable_intent(Some("cuda:0")),
                ),
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
