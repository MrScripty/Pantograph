use std::sync::Arc;

use pumas_library::models::{
    PumasArtifactConsumer, PumasArtifactLoadTarget, PumasArtifactLoadTargetDiagnostic,
    PumasArtifactLoadTargetResolutionMode, ResolveModelArtifactLoadTargetRequest,
    ResolveModelArtifactLoadTargetResponse,
};
use pumas_library::PumasError;
use thiserror::Error;

use crate::runtime_host_execution::ValidatedRuntimeHostExecutionRequest;

const PANTOGRAPH_RUNTIME_HOST_CONSUMER: &str = "pantograph-embedded-runtime";
const MAX_DIAGNOSTICS: usize = 4;

#[allow(dead_code)]
pub(crate) struct RuntimeHostPumasLoadTargetResolver {
    pumas_api: Arc<pumas_library::PumasApi>,
}

#[allow(dead_code)]
impl RuntimeHostPumasLoadTargetResolver {
    pub(crate) fn new(pumas_api: Arc<pumas_library::PumasApi>) -> Self {
        Self { pumas_api }
    }

    pub(crate) async fn resolve(
        &self,
        request: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<PumasArtifactLoadTarget, RuntimeHostPumasLoadTargetError> {
        let pumas_request = build_runtime_host_artifact_load_target_request(request)?;
        let response = self
            .pumas_api
            .resolve_model_artifact_load_target(pumas_request)
            .await?;
        ready_runtime_host_artifact_load_target(response)
    }
}

fn build_runtime_host_artifact_load_target_request(
    request: &ValidatedRuntimeHostExecutionRequest,
) -> Result<ResolveModelArtifactLoadTargetRequest, RuntimeHostPumasLoadTargetError> {
    let handoff = &request.as_ref().handoff;
    let dispatch_decision = handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostPumasLoadTargetError::MissingDispatchDecision)?;
    Ok(ResolveModelArtifactLoadTargetRequest {
        model_ref: pumas_library::models::PumasModelRef {
            model_id: dispatch_decision.selected_model_ref.model_id.clone(),
            revision: dispatch_decision.selected_model_ref.revision.clone(),
            selected_artifact_id: dispatch_decision
                .selected_model_ref
                .selected_artifact_id
                .clone(),
            selected_artifact_path: dispatch_decision
                .selected_model_ref
                .selected_artifact_path
                .clone(),
            migration_diagnostics: dispatch_decision
                .selected_model_ref
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
        caller_observed_entry_path: dispatch_decision
            .selected_model_ref
            .selected_artifact_path
            .clone(),
        caller_observed_package_facts_contract_version: None,
        resolution_mode: PumasArtifactLoadTargetResolutionMode::OwnerFresh,
        consumer: PumasArtifactConsumer {
            consumer_name: PANTOGRAPH_RUNTIME_HOST_CONSUMER.to_string(),
            task_kind: Some(handoff.task_intent.task_type.to_string()),
            runtime_family: Some(runtime_family(dispatch_decision)),
        },
    })
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
    #[error("ready Pumas artifact load-target response did not include a target")]
    ReadyResponseMissingTarget,
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
