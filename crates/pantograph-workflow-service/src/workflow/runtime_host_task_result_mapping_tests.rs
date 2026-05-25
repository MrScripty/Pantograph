use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionDiagnostic, RuntimeHostExecutionDiagnosticCode,
    RuntimeHostExecutionDiagnosticSeverity, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, ValidatedRuntimeHostExecutionResponse,
};

use super::{runtime_host_response_to_task_result, WorkflowRuntimeHostTaskResultMappingError};
use crate::workflow::{
    WorkflowSchedulerTaskResultDiagnosticSeverity, WorkflowSchedulerTaskResultStatus,
    WorkflowSchedulerTaskResultValue,
};

#[test]
fn completed_runtime_host_response_maps_to_completed_task_result() {
    let response = validated_response(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_response_completed_outputs.json"
    ));

    let result = runtime_host_response_to_task_result(&response)
        .expect("completed runtime-host response must map to task result");

    assert_eq!(result.workflow_id, "workflow.image_generation");
    assert_eq!(result.workflow_run_id, "run.2026-05-22.001");
    assert_eq!(result.node_id, "node.image_generation");
    assert_eq!(result.task_id, "task.image_generation.001");
    assert_eq!(result.status, WorkflowSchedulerTaskResultStatus::Completed);
    assert_eq!(result.outputs.len(), 2);
    assert_eq!(
        result.outputs[0].value,
        WorkflowSchedulerTaskResultValue::MediaArtifactRef(
            crate::workflow::WorkflowSchedulerTaskMediaArtifactRef {
                artifact_id: "artifact.image.001".to_owned(),
                media_type: Some("image_png".to_owned()),
            },
        )
    );
    assert_eq!(
        result.outputs[1].value,
        WorkflowSchedulerTaskResultValue::U64(42)
    );
    assert_eq!(
        result.diagnostics[0].severity,
        WorkflowSchedulerTaskResultDiagnosticSeverity::Info
    );
    assert_eq!(
        result.diagnostics[0].code,
        "runtime_host.execution_completed"
    );
    assert_eq!(result.terminal_metadata.expect("metadata").attempt, Some(1));
}

#[test]
fn accepted_runtime_host_response_does_not_materialize_task_result() {
    let response = validated_response(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_response_accepted.json"
    ));

    let error = runtime_host_response_to_task_result(&response)
        .expect_err("accepted runtime-host response is not a terminal task result");

    assert_eq!(
        error,
        WorkflowRuntimeHostTaskResultMappingError::NonTerminalRuntimeHostState
    );
}

#[test]
fn failed_runtime_host_response_maps_to_failed_task_result() {
    let mut response: RuntimeHostExecutionResponse = serde_json::from_str(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_response_accepted.json"
    ))
    .expect("runtime-host response fixture must decode");
    response.state = RuntimeHostExecutionState::Failed;
    response.diagnostics = vec![RuntimeHostExecutionDiagnostic {
        severity: RuntimeHostExecutionDiagnosticSeverity::Error,
        code: RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
        message: "Runtime execution failed.".to_owned(),
        hint: None,
    }];
    let response = ValidatedRuntimeHostExecutionResponse::try_from(response)
        .expect("failed runtime-host response must validate");

    let result = runtime_host_response_to_task_result(&response)
        .expect("failed runtime-host response must map to failed task result");

    assert_eq!(result.status, WorkflowSchedulerTaskResultStatus::Failed);
    assert!(result.outputs.is_empty());
    assert_eq!(result.diagnostics[0].code, "runtime_host.execution_failed");
    assert_eq!(
        result.diagnostics[0].severity,
        WorkflowSchedulerTaskResultDiagnosticSeverity::Error
    );
}

fn validated_response(json: &str) -> ValidatedRuntimeHostExecutionResponse {
    let response: RuntimeHostExecutionResponse =
        serde_json::from_str(json).expect("runtime-host response fixture must decode");
    ValidatedRuntimeHostExecutionResponse::try_from(response)
        .expect("runtime-host response fixture must validate")
}
