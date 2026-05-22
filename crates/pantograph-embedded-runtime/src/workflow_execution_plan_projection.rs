use inference::{
    BackendExecutionDecision, BackendExecutionSelectionPolicyTrace, BackendId,
    CapabilityAvailabilityId, CapabilityAvailabilityReason, CapabilityAvailabilityState,
    DependencyReadinessFact, DependencyReadinessResolverOwner, DeviceResolutionDecision,
    DeviceResolutionDiagnostic, DeviceResolutionDiagnosticCode, DeviceResolutionDiagnosticSeverity,
    InferenceDeviceClass, InferenceDeviceId, InferenceDevicePolicy, InferenceTaskId, PumasModelRef,
    RuntimeVariantId,
};
use pantograph_workflow_service::{
    WorkflowExecutionPlanDiagnostic, WorkflowExecutionPlanDiagnosticCode,
    WorkflowExecutionPlanDiagnosticSeverity, WorkflowExecutionPlanNodeDecision,
    WorkflowInferenceDeviceClass, WorkflowInferenceTaskId,
};
use pantograph_workflow_service::{
    WorkflowTechnicalFitDependencyReadinessFact,
    WorkflowTechnicalFitDependencyReadinessResolverOwner,
    WorkflowTechnicalFitDependencyReadinessState,
    WorkflowTechnicalFitDependencyReadinessSubjectKind,
};
use thiserror::Error;

pub(crate) fn project_workflow_node_decision_to_backend_execution_decision(
    decision: &WorkflowExecutionPlanNodeDecision,
) -> Result<BackendExecutionDecision, WorkflowExecutionPlanProjectionError> {
    let selected_backend_id =
        BackendId::parse(decision.selected_backend_key()).map_err(|error| {
            WorkflowExecutionPlanProjectionError::InvalidBackendId {
                value: decision.selected_backend_key().to_string(),
                message: error.to_string(),
            }
        })?;
    let selected_runtime_variant_id =
        RuntimeVariantId::parse(decision.selected_runtime_variant_id()).map_err(|error| {
            WorkflowExecutionPlanProjectionError::InvalidRuntimeVariantId {
                value: decision.selected_runtime_variant_id().to_string(),
                message: error.to_string(),
            }
        })?;
    let selected_device_class = project_device_class(decision.selected_device_class())?;
    let selected_device_id = decision
        .selected_device_id()
        .map(|device_id| {
            InferenceDeviceId::parse(device_id).map_err(|error| {
                WorkflowExecutionPlanProjectionError::InvalidDeviceId {
                    value: device_id.to_string(),
                    message: error.to_string(),
                }
            })
        })
        .transpose()?;
    let selected_task_id = project_task_id(decision.selected_task_id())?;
    let diagnostics = decision
        .diagnostics()
        .iter()
        .map(|diagnostic| project_diagnostic(diagnostic, &selected_backend_id))
        .collect::<Vec<_>>();
    let dependency_readiness = decision
        .dependency_readiness()
        .iter()
        .map(|fact| project_dependency_readiness_fact(fact, &selected_backend_id))
        .collect::<Result<Vec<_>, _>>()?;
    let selection_policy_trace = project_policy_trace(decision.policy_trace_ids());

    Ok(BackendExecutionDecision {
        selected_backend_id,
        selected_runtime_variant_id: selected_runtime_variant_id.clone(),
        selected_device_class,
        selected_device_id: selected_device_id.clone(),
        device_decision: DeviceResolutionDecision {
            policy: InferenceDevicePolicy::Auto,
            runtime_variant_id: selected_runtime_variant_id,
            selected_device_class,
            selected_device_id,
            diagnostics: diagnostics.clone(),
        },
        selected_task_id: Some(selected_task_id),
        selected_model_ref: decision.selected_model_ref().map(project_model_ref),
        diagnostics,
        dependency_readiness,
        selection_policy_trace,
    })
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum WorkflowExecutionPlanProjectionError {
    #[error("invalid selected backend id '{value}': {message}")]
    InvalidBackendId { value: String, message: String },
    #[error("invalid selected runtime variant id '{value}': {message}")]
    InvalidRuntimeVariantId { value: String, message: String },
    #[error("invalid selected device id '{value}': {message}")]
    InvalidDeviceId { value: String, message: String },
    #[error("unsupported selected device class {device_class}")]
    UnsupportedDeviceClass { device_class: &'static str },
    #[error("unsupported selected task id {task_id}")]
    UnsupportedTaskId { task_id: &'static str },
    #[error("invalid dependency readiness proof {field} '{value}': {message}")]
    InvalidDependencyReadinessProof {
        field: &'static str,
        value: String,
        message: String,
    },
}

fn project_dependency_readiness_fact(
    fact: &WorkflowTechnicalFitDependencyReadinessFact,
    selected_backend_id: &BackendId,
) -> Result<DependencyReadinessFact, WorkflowExecutionPlanProjectionError> {
    let runtime_id = fact
        .backend_key
        .as_deref()
        .map(parse_dependency_backend_id)
        .transpose()?
        .unwrap_or_else(|| selected_backend_id.clone());
    let dependency_id = parse_capability_id("dependency_id", fact.dependency_id.as_str())?;
    let mut projected = match fact.subject_kind {
        WorkflowTechnicalFitDependencyReadinessSubjectKind::Package => {
            DependencyReadinessFact::package(
                runtime_id,
                dependency_id,
                project_dependency_readiness_state(fact.state),
                project_dependency_readiness_owner(fact.resolver_owner),
            )
        }
        WorkflowTechnicalFitDependencyReadinessSubjectKind::Dependency => {
            DependencyReadinessFact::dependency(
                runtime_id,
                dependency_id,
                project_dependency_readiness_state(fact.state),
                project_dependency_readiness_owner(fact.resolver_owner),
            )
        }
    };

    if let Some(runtime_variant_id) = fact.runtime_variant_id.as_deref() {
        projected = projected.with_runtime_variant_id(
            RuntimeVariantId::parse(runtime_variant_id).map_err(|error| {
                WorkflowExecutionPlanProjectionError::InvalidDependencyReadinessProof {
                    field: "runtime_variant_id",
                    value: runtime_variant_id.to_string(),
                    message: error.to_string(),
                }
            })?,
        );
    }
    if let Some(task_id) = fact.task_id.as_deref() {
        projected = projected.with_task_id(project_dependency_task_id(task_id)?);
    }
    if let Some(model_family_id) = fact.model_family_id.as_deref() {
        projected = projected
            .with_model_family_id(parse_capability_id("model_family_id", model_family_id)?);
    }
    if let Some(reason_code) = fact.reason_code.as_deref() {
        projected = projected.with_reason_code(parse_capability_id("reason_code", reason_code)?);
    }
    if let Some(reason) = fact.reason.as_deref() {
        projected = projected.with_reason(CapabilityAvailabilityReason::parse(reason).map_err(
            |error| WorkflowExecutionPlanProjectionError::InvalidDependencyReadinessProof {
                field: "reason",
                value: reason.to_string(),
                message: error.to_string(),
            },
        )?);
    }

    Ok(projected)
}

fn parse_dependency_backend_id(
    backend_key: &str,
) -> Result<BackendId, WorkflowExecutionPlanProjectionError> {
    BackendId::parse(backend_key).map_err(|error| {
        WorkflowExecutionPlanProjectionError::InvalidDependencyReadinessProof {
            field: "backend_key",
            value: backend_key.to_string(),
            message: error.to_string(),
        }
    })
}

fn parse_capability_id(
    field: &'static str,
    value: &str,
) -> Result<CapabilityAvailabilityId, WorkflowExecutionPlanProjectionError> {
    CapabilityAvailabilityId::parse(value).map_err(|error| {
        WorkflowExecutionPlanProjectionError::InvalidDependencyReadinessProof {
            field,
            value: value.to_string(),
            message: error.to_string(),
        }
    })
}

fn project_dependency_task_id(
    task_id: &str,
) -> Result<InferenceTaskId, WorkflowExecutionPlanProjectionError> {
    match task_id {
        "text_generation" => Ok(InferenceTaskId::TextGeneration),
        "chat_completion" => Ok(InferenceTaskId::ChatCompletion),
        "embedding" => Ok(InferenceTaskId::Embedding),
        "rerank" => Ok(InferenceTaskId::Rerank),
        "image_generation" => Ok(InferenceTaskId::ImageGeneration),
        "image_understanding" => Ok(InferenceTaskId::ImageUnderstanding),
        "depth_estimation" => Ok(InferenceTaskId::DepthEstimation),
        "audio_transcription" => Ok(InferenceTaskId::AudioTranscription),
        "video_understanding" => Ok(InferenceTaskId::VideoUnderstanding),
        "multimodal_generation" => Ok(InferenceTaskId::MultimodalGeneration),
        _ => Err(
            WorkflowExecutionPlanProjectionError::InvalidDependencyReadinessProof {
                field: "task_id",
                value: task_id.to_string(),
                message: "unsupported inference task id".to_string(),
            },
        ),
    }
}

fn project_dependency_readiness_state(
    state: WorkflowTechnicalFitDependencyReadinessState,
) -> CapabilityAvailabilityState {
    match state {
        WorkflowTechnicalFitDependencyReadinessState::Available => {
            CapabilityAvailabilityState::Available
        }
        WorkflowTechnicalFitDependencyReadinessState::NotInstalled => {
            CapabilityAvailabilityState::NotInstalled
        }
        WorkflowTechnicalFitDependencyReadinessState::NotImplemented => {
            CapabilityAvailabilityState::NotImplemented
        }
        WorkflowTechnicalFitDependencyReadinessState::UnsupportedPlatform => {
            CapabilityAvailabilityState::UnsupportedPlatform
        }
        WorkflowTechnicalFitDependencyReadinessState::MissingDependency => {
            CapabilityAvailabilityState::MissingDependency
        }
        WorkflowTechnicalFitDependencyReadinessState::DisabledByPolicy => {
            CapabilityAvailabilityState::DisabledByPolicy
        }
        WorkflowTechnicalFitDependencyReadinessState::MissingModelFacts => {
            CapabilityAvailabilityState::MissingModelFacts
        }
        WorkflowTechnicalFitDependencyReadinessState::RequiresRuntimeCapability => {
            CapabilityAvailabilityState::RequiresRuntimeCapability
        }
        WorkflowTechnicalFitDependencyReadinessState::RequiresModelCapability => {
            CapabilityAvailabilityState::RequiresModelCapability
        }
    }
}

fn project_dependency_readiness_owner(
    owner: WorkflowTechnicalFitDependencyReadinessResolverOwner,
) -> DependencyReadinessResolverOwner {
    match owner {
        WorkflowTechnicalFitDependencyReadinessResolverOwner::Inference => {
            DependencyReadinessResolverOwner::Inference
        }
        WorkflowTechnicalFitDependencyReadinessResolverOwner::EmbeddedRuntime => {
            DependencyReadinessResolverOwner::EmbeddedRuntime
        }
        WorkflowTechnicalFitDependencyReadinessResolverOwner::ManagedRuntime => {
            DependencyReadinessResolverOwner::ManagedRuntime
        }
        WorkflowTechnicalFitDependencyReadinessResolverOwner::RuntimeBridge => {
            DependencyReadinessResolverOwner::RuntimeBridge
        }
    }
}

fn project_device_class(
    device_class: WorkflowInferenceDeviceClass,
) -> Result<InferenceDeviceClass, WorkflowExecutionPlanProjectionError> {
    match device_class {
        WorkflowInferenceDeviceClass::Cpu => Ok(InferenceDeviceClass::Cpu),
        WorkflowInferenceDeviceClass::Cuda => Ok(InferenceDeviceClass::Cuda),
        WorkflowInferenceDeviceClass::Metal => Ok(InferenceDeviceClass::Metal),
        WorkflowInferenceDeviceClass::Mps => Ok(InferenceDeviceClass::Mps),
        WorkflowInferenceDeviceClass::Unknown => Err(
            WorkflowExecutionPlanProjectionError::UnsupportedDeviceClass {
                device_class: "unknown",
            },
        ),
    }
}

fn project_task_id(
    task_id: WorkflowInferenceTaskId,
) -> Result<InferenceTaskId, WorkflowExecutionPlanProjectionError> {
    match task_id {
        WorkflowInferenceTaskId::TextGeneration => Ok(InferenceTaskId::TextGeneration),
        WorkflowInferenceTaskId::ChatCompletion => Ok(InferenceTaskId::ChatCompletion),
        WorkflowInferenceTaskId::Embedding => Ok(InferenceTaskId::Embedding),
        WorkflowInferenceTaskId::Rerank => Ok(InferenceTaskId::Rerank),
        WorkflowInferenceTaskId::ImageGeneration => Ok(InferenceTaskId::ImageGeneration),
        WorkflowInferenceTaskId::ImageUnderstanding => Ok(InferenceTaskId::ImageUnderstanding),
        WorkflowInferenceTaskId::DepthEstimation => Ok(InferenceTaskId::DepthEstimation),
        WorkflowInferenceTaskId::AudioTranscription => Ok(InferenceTaskId::AudioTranscription),
        WorkflowInferenceTaskId::VideoUnderstanding => Ok(InferenceTaskId::VideoUnderstanding),
        WorkflowInferenceTaskId::MultimodalGeneration => Ok(InferenceTaskId::MultimodalGeneration),
        WorkflowInferenceTaskId::Unknown => {
            Err(WorkflowExecutionPlanProjectionError::UnsupportedTaskId { task_id: "unknown" })
        }
    }
}

fn project_diagnostic(
    diagnostic: &WorkflowExecutionPlanDiagnostic,
    backend_id: &BackendId,
) -> DeviceResolutionDiagnostic {
    DeviceResolutionDiagnostic {
        code: project_diagnostic_code(diagnostic.code()),
        severity: project_diagnostic_severity(diagnostic.severity()),
        message: diagnostic.message().to_string(),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_id: Some(backend_id.clone()),
    }
}

fn project_diagnostic_code(
    code: WorkflowExecutionPlanDiagnosticCode,
) -> DeviceResolutionDiagnosticCode {
    match code {
        WorkflowExecutionPlanDiagnosticCode::MissingNodeDecision
        | WorkflowExecutionPlanDiagnosticCode::MissingSelectedDecisionFact => {
            DeviceResolutionDiagnosticCode::NoValidCandidate
        }
        WorkflowExecutionPlanDiagnosticCode::InvalidSelectedDecisionFact
        | WorkflowExecutionPlanDiagnosticCode::ProjectionFailed => {
            DeviceResolutionDiagnosticCode::BackendIncompatible
        }
        WorkflowExecutionPlanDiagnosticCode::AmbiguousNodeMapping => {
            DeviceResolutionDiagnosticCode::AmbiguousAutoResolution
        }
        WorkflowExecutionPlanDiagnosticCode::StaleRunContext => {
            DeviceResolutionDiagnosticCode::CandidateUnavailable
        }
        _ => DeviceResolutionDiagnosticCode::BackendIncompatible,
    }
}

fn project_diagnostic_severity(
    severity: WorkflowExecutionPlanDiagnosticSeverity,
) -> DeviceResolutionDiagnosticSeverity {
    match severity {
        WorkflowExecutionPlanDiagnosticSeverity::Info => {
            DeviceResolutionDiagnosticSeverity::Advisory
        }
        WorkflowExecutionPlanDiagnosticSeverity::Warning => {
            DeviceResolutionDiagnosticSeverity::Warning
        }
        WorkflowExecutionPlanDiagnosticSeverity::Error => DeviceResolutionDiagnosticSeverity::Error,
        _ => DeviceResolutionDiagnosticSeverity::Error,
    }
}

fn project_policy_trace(
    policy_trace_ids: &[String],
) -> Option<BackendExecutionSelectionPolicyTrace> {
    policy_trace_ids.iter().find_map(|trace_id| {
        trace_id
            .strip_prefix("technical_fit_policy_v")
            .and_then(|version| version.parse::<u32>().ok())
            .map(|policy_version| BackendExecutionSelectionPolicyTrace {
                policy_version,
                candidate_set_summary: None,
                ranking_reason: None,
                exploration_reason: None,
                seed_basis: Some(trace_id.clone()),
            })
    })
}

fn project_model_ref(model_ref: &str) -> PumasModelRef {
    PumasModelRef {
        model_id: model_ref.to_string(),
        revision: None,
        selected_artifact_id: None,
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    }
}

#[cfg(test)]
#[path = "workflow_execution_plan_projection_tests.rs"]
mod tests;
