use inference::{
    BackendExecutionDecision, BackendExecutionSelectionPolicyTrace, BackendId,
    DeviceResolutionDecision, DeviceResolutionDiagnostic, DeviceResolutionDiagnosticCode,
    DeviceResolutionDiagnosticSeverity, InferenceDeviceClass, InferenceDeviceId,
    InferenceDevicePolicy, InferenceTaskId, PumasModelRef, RuntimeVariantId,
};
use node_engine::planned_inference::PlannedInferenceDecisionContext;
use pantograph_workflow_service::{
    WorkflowExecutionPlan, WorkflowExecutionPlanDiagnostic, WorkflowExecutionPlanDiagnosticCode,
    WorkflowExecutionPlanDiagnosticSeverity, WorkflowExecutionPlanNodeDecision,
    WorkflowInferenceDeviceClass, WorkflowInferenceTaskId,
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
        selection_policy_trace,
    })
}

pub(crate) fn project_workflow_execution_plan_to_planned_inference_context(
    execution_plan: &WorkflowExecutionPlan,
) -> Result<PlannedInferenceDecisionContext, WorkflowExecutionPlanProjectionError> {
    let decisions = execution_plan
        .node_decisions()
        .iter()
        .map(|(node_id, decision)| {
            project_workflow_node_decision_to_backend_execution_decision(decision)
                .map(|backend_decision| (node_id.clone(), backend_decision))
        })
        .collect::<Result<std::collections::HashMap<_, _>, _>>()?;

    PlannedInferenceDecisionContext::new(execution_plan.workflow_run_id().as_str(), decisions)
        .map_err(
            |error| WorkflowExecutionPlanProjectionError::InvalidPlannedContext {
                message: error.to_string(),
            },
        )
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
    #[error("invalid planned inference context: {message}")]
    InvalidPlannedContext { message: String },
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
