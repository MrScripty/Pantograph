use std::sync::Arc;

use async_trait::async_trait;
use inference::{
    BackendExecutionDecision, ImageGenerationRequest, ImageGenerationResult,
    InferenceRequestLifecycleEventSink, InferenceTaskId, ModelArtifactKind, ModelStorageKind,
    ModelValidationState, PlannedImageGenerationLaunchHandoff, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, ResolvedModelPackageFacts,
};
use node_engine::planned_inference::{
    PlannedImageGenerationRequest, PlannedInferenceExecutionError, PlannedInferenceExecutionHost,
};
use pantograph_workflow_service::{WorkflowExecutionPlan, WorkflowServiceError};
use pumas_library::models::{
    AssetValidationState, PackageArtifactKind, PumasArtifactConsumer,
    PumasArtifactLoadTargetResolutionMode, ResolveModelArtifactLoadTargetRequest,
    ResolveModelArtifactLoadTargetResponse, StorageKind,
};
use thiserror::Error;

use crate::workflow_execution_plan_projection::project_workflow_node_decision_to_backend_execution_decision;
use crate::{RuntimeExtensionsSnapshot, SharedWorkflowService};

const PANTOGRAPH_CONSUMER_NAME: &str = "pantograph-embedded-runtime";
const IMAGE_GENERATION_TASK_KIND: &str = "image_generation";

pub(crate) struct EmbeddedPlannedInferenceExecutionHost {
    workflow_service: SharedWorkflowService,
    workflow_execution_session_id: String,
    gateway: Arc<inference::InferenceGateway>,
    runtime_extensions: RuntimeExtensionsSnapshot,
    lifecycle_sink: Option<Arc<dyn InferenceRequestLifecycleEventSink>>,
}

impl EmbeddedPlannedInferenceExecutionHost {
    pub(crate) fn new(
        workflow_service: SharedWorkflowService,
        workflow_execution_session_id: impl Into<String>,
        gateway: Arc<inference::InferenceGateway>,
        runtime_extensions: RuntimeExtensionsSnapshot,
        lifecycle_sink: Option<Arc<dyn InferenceRequestLifecycleEventSink>>,
    ) -> Self {
        Self {
            workflow_service,
            workflow_execution_session_id: workflow_execution_session_id.into(),
            gateway,
            runtime_extensions,
            lifecycle_sink,
        }
    }

    fn pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        self.runtime_extensions.pumas_api.clone()
    }
}

#[async_trait]
impl PlannedInferenceExecutionHost for EmbeddedPlannedInferenceExecutionHost {
    async fn generate_image(
        &self,
        request: PlannedImageGenerationRequest,
    ) -> Result<ImageGenerationResult, PlannedInferenceExecutionError> {
        let execution_plan = active_execution_plan(
            self.workflow_service.as_ref(),
            &self.workflow_execution_session_id,
            request.workflow_run_id(),
        )
        .map_err(|error| {
            PlannedInferenceExecutionError::execution_failed_with_source(
                &request,
                "failed to read active workflow execution plan",
                error,
            )
        })?
        .ok_or_else(|| {
            PlannedInferenceExecutionError::execution_failed(
                &request,
                "workflow run has no active scheduler execution plan",
            )
        })?;

        let backend_decision = backend_decision_for_node(&execution_plan, request.node_id())
            .map_err(|error| {
                PlannedInferenceExecutionError::execution_failed_with_source(
                    &request,
                    "failed to project scheduler decision for planned image generation",
                    error,
                )
            })?;
        let package_facts = self
            .resolve_package_facts(&request, &backend_decision)
            .await?;
        let artifact_load_target = self
            .resolve_artifact_load_target(&request, &backend_decision, &package_facts)
            .await?;
        let handoff = PlannedImageGenerationLaunchHandoff::new(
            package_facts,
            artifact_load_target,
            backend_decision,
        )
        .map_err(|error| {
            PlannedInferenceExecutionError::execution_failed_with_source(
                &request,
                "failed to build planned image-generation launch handoff",
                error,
            )
        })?;

        generate_image(
            self.gateway.as_ref(),
            request.image_request(),
            &handoff,
            request.request_id(),
            self.lifecycle_sink.clone(),
        )
        .await
        .map_err(|error| {
            PlannedInferenceExecutionError::execution_failed_with_source(
                &request,
                "planned image-generation gateway execution failed",
                error,
            )
        })
    }
}

impl EmbeddedPlannedInferenceExecutionHost {
    async fn resolve_package_facts(
        &self,
        request: &PlannedImageGenerationRequest,
        backend_decision: &BackendExecutionDecision,
    ) -> Result<ResolvedModelPackageFacts, PlannedInferenceExecutionError> {
        let pumas_api = self.pumas_api().ok_or_else(|| {
            PlannedInferenceExecutionError::execution_failed(
                request,
                "Pumas API extension is required for planned image generation",
            )
        })?;
        let selected_model_ref = backend_decision
            .selected_model_ref
            .as_ref()
            .ok_or_else(|| {
                PlannedInferenceExecutionError::execution_failed(
                    request,
                    "scheduler decision is missing selected Pumas model reference",
                )
            })?;
        let facts = pumas_api
            .resolve_model_package_facts(selected_model_ref.model_id.as_str())
            .await
            .map_err(|error| {
                PlannedInferenceExecutionError::execution_failed_with_source(
                    request,
                    "Pumas package facts resolution failed",
                    error,
                )
            })?;

        decode_pumas_package_facts(facts).map_err(|error| {
            PlannedInferenceExecutionError::execution_failed_with_source(
                request,
                "Pumas package facts could not be decoded into the inference contract",
                error,
            )
        })
    }

    async fn resolve_artifact_load_target(
        &self,
        request: &PlannedImageGenerationRequest,
        backend_decision: &BackendExecutionDecision,
        package_facts: &ResolvedModelPackageFacts,
    ) -> Result<PumasArtifactLoadTarget, PlannedInferenceExecutionError> {
        let pumas_api = self.pumas_api().ok_or_else(|| {
            PlannedInferenceExecutionError::execution_failed(
                request,
                "Pumas API extension is required for planned image generation",
            )
        })?;
        let selected_model_ref = backend_decision
            .selected_model_ref
            .as_ref()
            .ok_or_else(|| {
                PlannedInferenceExecutionError::execution_failed(
                    request,
                    "scheduler decision is missing selected Pumas model reference",
                )
            })?;
        let artifact_request = build_image_artifact_load_target_request(
            selected_model_ref,
            backend_decision,
            package_facts,
        );
        let response = pumas_api
            .resolve_model_artifact_load_target(artifact_request)
            .await
            .map_err(|error| {
                PlannedInferenceExecutionError::execution_failed_with_source(
                    request,
                    "Pumas artifact load-target resolution failed",
                    error,
                )
            })?;
        let target = ready_pumas_artifact_load_target(response).map_err(|error| {
            PlannedInferenceExecutionError::execution_failed_with_source(
                request,
                "Pumas artifact load target is not ready for image generation",
                error,
            )
        })?;

        Ok(project_pumas_artifact_load_target(target))
    }
}

fn active_execution_plan(
    workflow_service: &pantograph_workflow_service::WorkflowService,
    workflow_execution_session_id: &str,
    workflow_run_id: &str,
) -> Result<Option<WorkflowExecutionPlan>, WorkflowServiceError> {
    workflow_service.workflow_execution_session_active_execution_plan(
        workflow_execution_session_id,
        workflow_run_id,
    )
}

fn backend_decision_for_node(
    execution_plan: &WorkflowExecutionPlan,
    node_id: &str,
) -> Result<BackendExecutionDecision, EmbeddedPlannedInferenceHostError> {
    let decision = execution_plan
        .node_decisions()
        .get(node_id)
        .ok_or_else(|| EmbeddedPlannedInferenceHostError::MissingNodeDecision {
            workflow_run_id: execution_plan.workflow_run_id().as_str().to_string(),
            node_id: node_id.to_string(),
        })?;
    let backend_decision = project_workflow_node_decision_to_backend_execution_decision(decision)?;
    if backend_decision.selected_task_id != Some(InferenceTaskId::ImageGeneration) {
        return Err(EmbeddedPlannedInferenceHostError::TaskMismatch {
            node_id: node_id.to_string(),
            actual_task_id: backend_decision.selected_task_id,
        });
    }
    Ok(backend_decision)
}

async fn generate_image(
    gateway: &inference::InferenceGateway,
    image_request: &ImageGenerationRequest,
    handoff: &PlannedImageGenerationLaunchHandoff,
    request_id: &str,
    lifecycle_sink: Option<Arc<dyn InferenceRequestLifecycleEventSink>>,
) -> Result<ImageGenerationResult, inference::GatewayError> {
    if let Some(lifecycle_sink) = lifecycle_sink {
        gateway
            .generate_image_from_launch_handoff_with_lifecycle(
                image_request,
                handoff,
                Some(request_id.to_string()),
                lifecycle_sink,
            )
            .await
    } else {
        gateway
            .generate_image_from_launch_handoff(image_request, handoff)
            .await
    }
}

fn build_image_artifact_load_target_request(
    selected_model_ref: &inference::PumasModelRef,
    backend_decision: &BackendExecutionDecision,
    package_facts: &ResolvedModelPackageFacts,
) -> ResolveModelArtifactLoadTargetRequest {
    ResolveModelArtifactLoadTargetRequest {
        model_ref: project_pumas_model_ref_to_pumas(selected_model_ref),
        expected_artifact_kind: Some(PackageArtifactKind::DiffusersBundle),
        caller_observed_entry_path: Some(package_facts.artifact.entry_path.clone()),
        caller_observed_package_facts_contract_version: Some(
            package_facts.package_facts_contract_version,
        ),
        resolution_mode: PumasArtifactLoadTargetResolutionMode::OwnerFresh,
        consumer: PumasArtifactConsumer {
            consumer_name: PANTOGRAPH_CONSUMER_NAME.to_string(),
            task_kind: Some(IMAGE_GENERATION_TASK_KIND.to_string()),
            runtime_family: Some(
                backend_decision
                    .selected_runtime_variant_id
                    .as_str()
                    .to_string(),
            ),
        },
    }
}

fn ready_pumas_artifact_load_target(
    response: ResolveModelArtifactLoadTargetResponse,
) -> Result<pumas_library::models::PumasArtifactLoadTarget, EmbeddedPlannedInferenceHostError> {
    if response.is_ready() {
        return response
            .target
            .ok_or(EmbeddedPlannedInferenceHostError::ReadyResponseMissingTarget);
    }

    Err(
        EmbeddedPlannedInferenceHostError::ArtifactLoadTargetUnavailable {
            artifact_state: format!("{:?}", response.artifact_state),
            entry_path_state: format!("{:?}", response.entry_path_state),
            diagnostics: response
                .diagnostics
                .iter()
                .take(4)
                .map(|diagnostic| format!("{:?}: {}", diagnostic.code, diagnostic.message))
                .collect(),
            diagnostic_count: response.diagnostics.len(),
        },
    )
}

fn decode_pumas_package_facts(
    facts: pumas_library::models::ResolvedModelPackageFacts,
) -> Result<ResolvedModelPackageFacts, serde_json::Error> {
    let mut value = serde_json::to_value(facts)?;
    strip_pumas_model_ref_contract_versions(&mut value);
    serde_json::from_value(value)
}

fn strip_pumas_model_ref_contract_versions(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if map.contains_key("model_id") {
                map.remove("model_ref_contract_version");
            }
            for child in map.values_mut() {
                strip_pumas_model_ref_contract_versions(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_pumas_model_ref_contract_versions(item);
            }
        }
        _ => {}
    }
}

fn project_pumas_artifact_load_target(
    target: pumas_library::models::PumasArtifactLoadTarget,
) -> PumasArtifactLoadTarget {
    PumasArtifactLoadTarget {
        model_ref: project_pumas_model_ref_from_pumas(target.model_ref),
        artifact_kind: project_artifact_kind(target.artifact_kind),
        local_load_path: target.local_load_path,
        load_path_kind: project_load_path_kind(target.load_path_kind),
        library_root_id: target.library_root_id,
        storage_kind: project_storage_kind(target.storage_kind),
        validation_state: project_validation_state(target.validation_state),
        content_fingerprint: target.content_fingerprint,
        package_facts_contract_version: target.package_facts_contract_version,
    }
}

fn project_pumas_model_ref_to_pumas(
    model_ref: &inference::PumasModelRef,
) -> pumas_library::models::PumasModelRef {
    pumas_library::models::PumasModelRef {
        model_id: model_ref.model_id.clone(),
        revision: model_ref.revision.clone(),
        selected_artifact_id: model_ref.selected_artifact_id.clone(),
        selected_artifact_path: model_ref.selected_artifact_path.clone(),
        migration_diagnostics: model_ref
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
    }
}

fn project_pumas_model_ref_from_pumas(
    model_ref: pumas_library::models::PumasModelRef,
) -> inference::PumasModelRef {
    inference::PumasModelRef {
        model_id: model_ref.model_id,
        revision: model_ref.revision,
        selected_artifact_id: model_ref.selected_artifact_id,
        selected_artifact_path: model_ref.selected_artifact_path,
        migration_diagnostics: model_ref
            .migration_diagnostics
            .into_iter()
            .map(|diagnostic| inference::ModelRefMigrationDiagnostic {
                code: diagnostic.code,
                message: diagnostic.message,
                input: diagnostic.input,
            })
            .collect(),
    }
}

fn project_artifact_kind(kind: PackageArtifactKind) -> ModelArtifactKind {
    match kind {
        PackageArtifactKind::Gguf => ModelArtifactKind::Gguf,
        PackageArtifactKind::HfCompatibleDirectory => ModelArtifactKind::HfCompatibleDirectory,
        PackageArtifactKind::Safetensors => ModelArtifactKind::Safetensors,
        PackageArtifactKind::DiffusersBundle => ModelArtifactKind::DiffusersBundle,
        PackageArtifactKind::Onnx => ModelArtifactKind::Onnx,
        PackageArtifactKind::Adapter => ModelArtifactKind::Adapter,
        PackageArtifactKind::Shard => ModelArtifactKind::Shard,
        PackageArtifactKind::Unknown => ModelArtifactKind::Unknown,
    }
}

fn project_load_path_kind(
    kind: pumas_library::models::PumasArtifactLoadPathKind,
) -> PumasArtifactLoadPathKind {
    match kind {
        pumas_library::models::PumasArtifactLoadPathKind::Directory => {
            PumasArtifactLoadPathKind::Directory
        }
        pumas_library::models::PumasArtifactLoadPathKind::File => PumasArtifactLoadPathKind::File,
    }
}

fn project_storage_kind(kind: StorageKind) -> ModelStorageKind {
    match kind {
        StorageKind::LibraryOwned => ModelStorageKind::LibraryOwned,
        StorageKind::ExternalReference => ModelStorageKind::ExternalReference,
    }
}

fn project_validation_state(state: AssetValidationState) -> ModelValidationState {
    match state {
        AssetValidationState::Valid => ModelValidationState::Valid,
        AssetValidationState::Degraded => ModelValidationState::Degraded,
        AssetValidationState::Invalid => ModelValidationState::Invalid,
    }
}

#[derive(Debug, Error)]
enum EmbeddedPlannedInferenceHostError {
    #[error("workflow run '{workflow_run_id}' has no scheduler decision for node '{node_id}'")]
    MissingNodeDecision {
        workflow_run_id: String,
        node_id: String,
    },
    #[error("scheduler decision for node '{node_id}' is not image generation")]
    TaskMismatch {
        node_id: String,
        actual_task_id: Option<InferenceTaskId>,
    },
    #[error("ready Pumas artifact load-target response did not include a target")]
    ReadyResponseMissingTarget,
    #[error(
        "Pumas artifact load target unavailable: artifact_state={artifact_state}, entry_path_state={entry_path_state}, diagnostics={diagnostic_count}"
    )]
    ArtifactLoadTargetUnavailable {
        artifact_state: String,
        entry_path_state: String,
        diagnostics: Vec<String>,
        diagnostic_count: usize,
    },
    #[error(transparent)]
    WorkflowExecutionPlanProjection(
        #[from] crate::workflow_execution_plan_projection::WorkflowExecutionPlanProjectionError,
    ),
}

#[cfg(test)]
#[path = "planned_inference_host_tests.rs"]
mod tests;
