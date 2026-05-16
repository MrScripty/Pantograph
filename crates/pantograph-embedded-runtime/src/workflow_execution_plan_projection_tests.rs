use inference::{
    DeviceResolutionDiagnosticCode, DeviceResolutionDiagnosticSeverity, InferenceDeviceClass,
    InferenceTaskId,
};
use pantograph_workflow_service::{
    WorkflowExecutionPlan, WorkflowExecutionPlanDiagnostic, WorkflowExecutionPlanDiagnosticCode,
    WorkflowExecutionPlanDiagnosticSeverity, WorkflowExecutionPlanError,
    WorkflowExecutionPlanNodeDecision, WorkflowInferenceDeviceClass, WorkflowInferenceTaskId,
};
use pantograph_workflow_service::{WorkflowId, WorkflowRunId};

use super::{
    project_workflow_execution_plan_to_planned_inference_context,
    project_workflow_node_decision_to_backend_execution_decision,
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
fn workflow_node_decision_projects_canonicalized_raw_model_ref() {
    let decision = WorkflowExecutionPlanNodeDecision::new(
        "image-node-1",
        "pytorch",
        "pytorch-runtime",
        "pytorch.cuda",
        WorkflowInferenceDeviceClass::Cuda,
        WorkflowInferenceTaskId::ImageGeneration,
    )
    .expect("valid node decision")
    .with_selected_model_ref("stable-diffusion-xl")
    .expect("raw selected model id should canonicalize at workflow boundary");

    let backend_decision = project_workflow_node_decision_to_backend_execution_decision(&decision)
        .expect("project backend decision");

    assert_eq!(
        backend_decision
            .selected_model_ref
            .as_ref()
            .map(|model_ref| model_ref.model_id.as_str()),
        Some("pumas://models/stable-diffusion-xl")
    );
}

#[test]
fn workflow_node_decision_rejects_invalid_backend_id() {
    let error = WorkflowExecutionPlanNodeDecision::new(
        "image-node-1",
        "llama.cpp",
        "llama-runtime",
        "llama_cpp.cuda",
        WorkflowInferenceDeviceClass::Cuda,
        WorkflowInferenceTaskId::ImageGeneration,
    )
    .expect_err("invalid backend id should fail at workflow boundary");

    assert!(matches!(
        error,
        WorkflowExecutionPlanError::InvalidSelectedDecisionFact {
            field: "selected_backend_key",
            ..
        }
    ));
}

#[test]
fn workflow_node_decision_rejects_invalid_device_id() {
    let error = WorkflowExecutionPlanNodeDecision::new(
        "image-node-1",
        "pytorch",
        "pytorch-runtime",
        "pytorch.cuda",
        WorkflowInferenceDeviceClass::Cuda,
        WorkflowInferenceTaskId::ImageGeneration,
    )
    .expect("valid node decision")
    .with_selected_device_id("CUDA:0")
    .expect_err("invalid device id should fail at workflow boundary");

    assert!(matches!(
        error,
        WorkflowExecutionPlanError::InvalidSelectedDecisionFact {
            field: "selected_device_id",
            ..
        }
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

#[test]
fn workflow_execution_plan_projects_to_node_engine_context() {
    let execution_plan = WorkflowExecutionPlan::new(
        WorkflowRunId::try_from("run-image-plan".to_string()).expect("workflow run id"),
        WorkflowId::try_from("workflow-image".to_string()).expect("workflow id"),
        vec![workflow_node_decision()],
    )
    .expect("execution plan");

    let context = project_workflow_execution_plan_to_planned_inference_context(&execution_plan)
        .expect("planned inference context");

    let decision = context
        .decision_for_node(
            "run-image-plan",
            "image-node-1",
            InferenceTaskId::ImageGeneration,
        )
        .expect("planned node decision");
    assert_eq!(context.workflow_run_id(), "run-image-plan");
    assert_eq!(decision.selected_backend_id.as_str(), "pytorch");
}
