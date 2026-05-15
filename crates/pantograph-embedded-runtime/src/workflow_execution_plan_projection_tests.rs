use inference::{
    DeviceResolutionDiagnosticCode, DeviceResolutionDiagnosticSeverity, InferenceDeviceClass,
    InferenceTaskId,
};
use pantograph_workflow_service::{
    WorkflowExecutionPlanDiagnostic, WorkflowExecutionPlanDiagnosticCode,
    WorkflowExecutionPlanDiagnosticSeverity, WorkflowExecutionPlanNodeDecision,
    WorkflowInferenceDeviceClass, WorkflowInferenceTaskId,
};

use super::{
    project_workflow_node_decision_to_backend_execution_decision,
    WorkflowExecutionPlanProjectionError,
};

fn workflow_node_decision() -> WorkflowExecutionPlanNodeDecision {
    WorkflowExecutionPlanNodeDecision::new(
        "image-node-1",
        "pytorch",
        "pytorch-runtime",
        "pytorch.cuda",
        WorkflowInferenceDeviceClass::Cuda,
        WorkflowInferenceTaskId::ImageGeneration,
    )
    .expect("valid node decision")
    .with_selected_device_id("cuda:0")
    .expect("valid selected device id")
    .with_selected_model_ref("pumas://models/stable-diffusion-xl")
    .expect("valid selected model ref")
    .with_policy_trace_ids(vec!["technical_fit_policy_v3".to_string()])
    .expect("valid policy trace")
}

#[test]
fn workflow_node_decision_projects_to_backend_execution_decision() {
    let backend_decision =
        project_workflow_node_decision_to_backend_execution_decision(&workflow_node_decision())
            .expect("project backend decision");

    assert_eq!(backend_decision.selected_backend_id.as_str(), "pytorch");
    assert_eq!(
        backend_decision.selected_runtime_variant_id.as_str(),
        "pytorch.cuda"
    );
    assert_eq!(
        backend_decision.selected_device_class,
        InferenceDeviceClass::Cuda
    );
    assert_eq!(
        backend_decision
            .selected_device_id
            .as_ref()
            .map(|device_id| device_id.as_str()),
        Some("cuda:0")
    );
    assert_eq!(
        backend_decision.selected_task_id,
        Some(InferenceTaskId::ImageGeneration)
    );
    assert_eq!(
        backend_decision
            .selected_model_ref
            .as_ref()
            .map(|model_ref| model_ref.model_id.as_str()),
        Some("pumas://models/stable-diffusion-xl")
    );
    assert_eq!(
        backend_decision
            .selection_policy_trace
            .as_ref()
            .map(|trace| trace.policy_version),
        Some(3)
    );
}

#[test]
fn workflow_node_decision_rejects_invalid_backend_id() {
    let decision = WorkflowExecutionPlanNodeDecision::new(
        "image-node-1",
        "llama.cpp",
        "llama-runtime",
        "llama_cpp.cuda",
        WorkflowInferenceDeviceClass::Cuda,
        WorkflowInferenceTaskId::ImageGeneration,
    )
    .expect("workflow decision allows reduced backend key text");

    let error = project_workflow_node_decision_to_backend_execution_decision(&decision)
        .expect_err("invalid backend id should fail projection");

    assert!(matches!(
        error,
        WorkflowExecutionPlanProjectionError::InvalidBackendId { .. }
    ));
}

#[test]
fn workflow_node_decision_rejects_invalid_device_id() {
    let decision = WorkflowExecutionPlanNodeDecision::new(
        "image-node-1",
        "pytorch",
        "pytorch-runtime",
        "pytorch.cuda",
        WorkflowInferenceDeviceClass::Cuda,
        WorkflowInferenceTaskId::ImageGeneration,
    )
    .expect("valid node decision")
    .with_selected_device_id("CUDA:0")
    .expect("workflow decision only validates bounded text");

    let error = project_workflow_node_decision_to_backend_execution_decision(&decision)
        .expect_err("invalid device id should fail projection");

    assert!(matches!(
        error,
        WorkflowExecutionPlanProjectionError::InvalidDeviceId { .. }
    ));
}

#[test]
fn workflow_node_decision_projects_diagnostics() {
    let diagnostic = WorkflowExecutionPlanDiagnostic::new(
        WorkflowExecutionPlanDiagnosticCode::AmbiguousNodeMapping,
        WorkflowExecutionPlanDiagnosticSeverity::Warning,
        "selected model maps to multiple image nodes",
    )
    .expect("valid diagnostic");
    let decision = workflow_node_decision()
        .with_diagnostics(vec![diagnostic])
        .expect("attach diagnostic");

    let backend_decision = project_workflow_node_decision_to_backend_execution_decision(&decision)
        .expect("project backend decision");

    assert_eq!(backend_decision.diagnostics.len(), 1);
    assert_eq!(
        backend_decision.diagnostics[0].code,
        DeviceResolutionDiagnosticCode::AmbiguousAutoResolution
    );
    assert_eq!(
        backend_decision.diagnostics[0].severity,
        DeviceResolutionDiagnosticSeverity::Warning
    );
    assert_eq!(
        backend_decision.diagnostics[0]
            .backend_id
            .as_ref()
            .map(|backend_id| backend_id.as_str()),
        Some("pytorch")
    );
}
