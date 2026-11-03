use std::sync::Arc;

use async_trait::async_trait;
use inference::PumasArtifactLoadTarget;
use pantograph_runtime_host_contracts::ValidatedRuntimeHostExecutionRequest;
use pumas_library::intent::{
    AcquisitionPolicy, ModelRequirement, ModelSelector, ObservedModelState,
};
use pumas_library::models::{
    PumasArtifactConsumer, PumasArtifactLoadTargetDiagnostic,
    PumasArtifactLoadTargetResolutionMode, ResolveModelArtifactLoadTargetRequest,
    ResolveModelArtifactLoadTargetResponse,
};
use pumas_library::PumasError;
use thiserror::Error;
use workflow_nodes::setup::{
    intent_handle_to_load_target, pumas_load_target_to_inference, summarize_intent_observation,
    IntentAvailabilityState, PumasSelectorAccess,
};

use crate::runtime_host_package_facts::pumas_library_model_id;

const PANTOGRAPH_RUNTIME_HOST_CONSUMER: &str = "pantograph-embedded-runtime";
const MAX_DIAGNOSTICS: usize = 4;

pub(crate) struct RuntimeHostPumasLoadTargetResolver {
    selector_access: Arc<PumasSelectorAccess>,
}

#[async_trait]
pub(crate) trait RuntimeHostLoadTargetResolver: Send + Sync {
    async fn resolve(
        &self,
        request: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<PumasArtifactLoadTarget, RuntimeHostPumasLoadTargetError>;
}

impl RuntimeHostPumasLoadTargetResolver {
    pub(crate) fn new(selector_access: Arc<PumasSelectorAccess>) -> Self {
        Self { selector_access }
    }

    /// Resolve an executable target without inventing task admission authority.
    ///
    /// Library-serving access resolves through the released Pumas intent
    /// `get` path and consumes only an `Available` handle; read-only access has
    /// no intent surface and keeps the owner-fresh load-target request.
    pub(crate) async fn resolve_for_model(
        &self,
        selected: &pantograph_dependency_planning::PumasModelRef,
        task_kind: &str,
        runtime_family: &str,
    ) -> Result<PumasArtifactLoadTarget, RuntimeHostPumasLoadTargetError> {
        let mut target = if matches!(
            self.selector_access.as_ref(),
            PumasSelectorAccess::ReadOnly(_)
        ) {
            let pumas_request =
                build_model_artifact_load_target_request(selected, task_kind, runtime_family);
            let response = self
                .selector_access
                .resolve_model_artifact_load_target(pumas_request)
                .await?;
            ready_runtime_host_artifact_load_target(response)?
        } else {
            let requirement = build_runtime_host_model_requirement(selected)?;
            let observation = self
                .selector_access
                .intent_get_model_with_targeted_hydration(requirement)
                .await?;
            match observation.state {
                ObservedModelState::Available { handle } => intent_handle_to_load_target(handle)?,
                state => {
                    let summary = summarize_intent_observation(
                        &workflow_nodes::setup::IntentAvailabilityObservation {
                            state,
                            hydration_error: observation.hydration_error,
                        },
                    );
                    return Err(RuntimeHostPumasLoadTargetError::IntentUnavailable {
                        state: summary.state,
                        candidate_count: summary.candidate_count,
                        diagnostics: summary.diagnostics,
                        hydration_error: summary.hydration_error,
                    });
                }
            }
        };
        validate_runtime_host_producer_target_identity(selected, &target)?;
        let selected_artifact_id = selected
            .selected_artifact_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let target_artifact_id = target
            .model_ref
            .selected_artifact_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(RuntimeHostPumasLoadTargetError::MissingSelectedArtifactIdentity)?;
        if let Some(selected_artifact_id) = selected_artifact_id {
            if selected_artifact_id != target_artifact_id {
                return Err(
                    RuntimeHostPumasLoadTargetError::SelectedArtifactIdentityMismatch {
                        selected_artifact_id: selected_artifact_id.to_string(),
                        target_artifact_id: target_artifact_id.to_string(),
                    },
                );
            }
        }
        if selected
            .model_id
            .strip_prefix("pumas://models/")
            .unwrap_or(&selected.model_id)
            != target
                .model_ref
                .model_id
                .strip_prefix("pumas://models/")
                .unwrap_or(&target.model_ref.model_id)
            || selected
                .revision
                .as_ref()
                .is_some_and(|revision| target.model_ref.revision.as_ref() != Some(revision))
            || selected
                .selected_artifact_id
                .as_ref()
                .is_some_and(|id| target.model_ref.selected_artifact_id.as_ref() != Some(id))
            || selected
                .selected_artifact_path
                .as_ref()
                .is_some_and(|path| target.model_ref.selected_artifact_path.as_ref() != Some(path))
        {
            return Err(RuntimeHostPumasLoadTargetError::SelectedIdentityMismatch);
        }
        // The physical executable path remains only in this host-owned target.
        // Scheduler refs omit path data; preserve their canonical identity shape.
        target.model_ref.selected_artifact_path = selected.selected_artifact_path.clone();
        Ok(target)
    }
}

#[async_trait]
impl RuntimeHostLoadTargetResolver for RuntimeHostPumasLoadTargetResolver {
    async fn resolve(
        &self,
        request: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<PumasArtifactLoadTarget, RuntimeHostPumasLoadTargetError> {
        let handoff = &request.as_ref().handoff;
        let decision = handoff
            .dispatch_decision
            .as_ref()
            .ok_or(RuntimeHostPumasLoadTargetError::MissingDispatchDecision)?;
        self.resolve_for_model(
            &decision.selected_model_ref,
            handoff.task_intent.task_type.as_str(),
            &runtime_family(decision),
        )
        .await
    }
}

fn build_runtime_host_model_requirement(
    selected_model_ref: &pantograph_dependency_planning::PumasModelRef,
) -> Result<ModelRequirement, RuntimeHostPumasLoadTargetError> {
    let model_ref = pumas_library::models::PumasModelRef {
        model_id: pumas_library_model_id(&selected_model_ref.model_id),
        revision: selected_model_ref.revision.clone(),
        selected_artifact_id: selected_model_ref.selected_artifact_id.clone(),
        // Pumas intent identity is path-free. The producer-issued executable
        // path is consumed only after an Available handle crosses the runtime
        // boundary; forwarding an authored/legacy path here makes it an
        // intent constraint and can turn a valid local model into an
        // artifact-mismatch result.
        selected_artifact_path: None,
        migration_diagnostics: selected_model_ref
            .migration_diagnostics
            .iter()
            .map(
                |diagnostic| pumas_library::models::ModelRefMigrationDiagnostic {
                    code: diagnostic.code.clone(),
                    message: diagnostic.message.clone(),
                    input: diagnostic.input.clone(),
                },
            )
            .collect(),
        ..Default::default()
    };
    Ok(ModelRequirement {
        selector: ModelSelector::LocalModel { model_ref },
        artifact: Default::default(),
        acquisition_policy: AcquisitionPolicy::LocalOnly,
    })
}

fn validate_runtime_host_producer_target_identity(
    selected_model_ref: &pantograph_dependency_planning::PumasModelRef,
    target: &PumasArtifactLoadTarget,
) -> Result<(), RuntimeHostPumasLoadTargetError> {
    let selected_model_id = pumas_library_model_id(selected_model_ref.model_id.as_str());
    let producer_model_id = pumas_library_model_id(target.model_ref.model_id.as_str());
    if selected_model_id.trim_matches('/') != producer_model_id.trim_matches('/') {
        return Err(RuntimeHostPumasLoadTargetError::ProducerModelMismatch {
            selected_model_id: selected_model_ref.model_id.clone(),
            producer_model_id: target.model_ref.model_id.clone(),
        });
    }
    if let (Some(selected_revision), Some(producer_revision)) = (
        selected_model_ref.revision.as_ref(),
        target.model_ref.revision.as_ref(),
    ) {
        if selected_revision != producer_revision {
            return Err(RuntimeHostPumasLoadTargetError::ProducerRevisionMismatch {
                selected_model_id: selected_model_ref.model_id.clone(),
                selected_revision: selected_revision.clone(),
                producer_revision: producer_revision.clone(),
            });
        }
    }
    Ok(())
}

fn build_model_artifact_load_target_request(
    selected: &pantograph_dependency_planning::PumasModelRef,
    task_kind: &str,
    runtime_family: &str,
) -> ResolveModelArtifactLoadTargetRequest {
    ResolveModelArtifactLoadTargetRequest {
        model_ref: pumas_library::models::PumasModelRef {
            model_id: selected.model_id.clone(),
            revision: selected.revision.clone(),
            selected_artifact_id: selected.selected_artifact_id.clone(),
            selected_artifact_path: selected.selected_artifact_path.clone(),
            migration_diagnostics: selected
                .migration_diagnostics
                .iter()
                .map(
                    |diagnostic| pumas_library::models::ModelRefMigrationDiagnostic {
                        code: diagnostic.code.clone(),
                        message: diagnostic.message.clone(),
                        input: diagnostic.input.clone(),
                    },
                )
                .collect(),
            ..Default::default()
        },
        expected_artifact_kind: None,
        caller_observed_entry_path: selected.selected_artifact_path.clone(),
        caller_observed_package_facts_contract_version: None,
        resolution_mode: PumasArtifactLoadTargetResolutionMode::OwnerFresh,
        consumer: PumasArtifactConsumer {
            consumer_name: PANTOGRAPH_RUNTIME_HOST_CONSUMER.to_string(),
            task_kind: Some(task_kind.to_owned()),
            runtime_family: Some(runtime_family.to_owned()),
        },
    }
}

fn runtime_family(decision: &pantograph_scheduler::SchedulerDispatchDecision) -> String {
    decision.selected_runtime_variant_id.as_ref().map_or_else(
        || decision.selected_runtime_id.to_string(),
        std::string::ToString::to_string,
    )
}

fn ready_runtime_host_artifact_load_target(
    response: ResolveModelArtifactLoadTargetResponse,
) -> Result<PumasArtifactLoadTarget, RuntimeHostPumasLoadTargetError> {
    if response.is_ready() {
        return response
            .target
            .map(pumas_load_target_to_inference)
            .ok_or(RuntimeHostPumasLoadTargetError::ReadyResponseMissingTarget);
    }
    Err(RuntimeHostPumasLoadTargetError::Unavailable {
        artifact_state: format!("{:?}", response.artifact_state),
        entry_path_state: format!("{:?}", response.entry_path_state),
        diagnostics: compact_diagnostics(&response.diagnostics),
        diagnostic_count: response.diagnostics.len(),
    })
}

fn compact_diagnostics(diagnostics: &[PumasArtifactLoadTargetDiagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .take(MAX_DIAGNOSTICS)
        .map(|diagnostic| format!("{:?}: {}", diagnostic.code, diagnostic.message))
        .collect()
}

#[derive(Debug, Error)]
pub(crate) enum RuntimeHostPumasLoadTargetError {
    #[error("runtime host execution request is missing scheduler dispatch decision")]
    MissingDispatchDecision,
    #[error("Pumas load target disagrees with selected model/revision/artifact identity")]
    SelectedIdentityMismatch,
    #[error("ready Pumas artifact load-target response did not include a target")]
    ReadyResponseMissingTarget,
    #[error("runtime host execution requires a producer-selected artifact identity")]
    MissingSelectedArtifactIdentity,
    #[error(
        "Pumas artifact load target model '{producer_model_id}' does not match scheduler model '{selected_model_id}'"
    )]
    ProducerModelMismatch {
        selected_model_id: String,
        producer_model_id: String,
    },
    #[error(
        "Pumas artifact load target revision '{producer_revision}' does not match scheduler revision '{selected_revision}' for model '{selected_model_id}'"
    )]
    ProducerRevisionMismatch {
        selected_model_id: String,
        selected_revision: String,
        producer_revision: String,
    },
    #[error(
        "Pumas artifact load target selected artifact identity '{target_artifact_id}' does not match scheduler identity '{selected_artifact_id}'"
    )]
    SelectedArtifactIdentityMismatch {
        selected_artifact_id: String,
        target_artifact_id: String,
    },
    #[error(
        "Pumas intent did not return an available model handle: state={state:?}, candidates={candidate_count}, diagnostics={diagnostics:?}, hydration_error={hydration_error:?}"
    )]
    IntentUnavailable {
        state: IntentAvailabilityState,
        candidate_count: usize,
        diagnostics: Vec<String>,
        hydration_error: Option<String>,
    },
    #[error(
        "Pumas artifact load target unavailable: artifact_state={artifact_state}, entry_path_state={entry_path_state}, diagnostics={diagnostic_count}"
    )]
    Unavailable {
        artifact_state: String,
        entry_path_state: String,
        diagnostics: Vec<String>,
        diagnostic_count: usize,
    },
    #[error(transparent)]
    Pumas(#[from] PumasError),
}

#[cfg(test)]
#[path = "runtime_host_load_target_tests.rs"]
mod tests;
