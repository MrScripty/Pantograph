use pantograph_scheduler::SchedulerRuntimeHandoffState;
use serde_json::json;

use super::{
    RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionMemberResponse,
    RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionRequest,
    RuntimeHostBatchExecutionResponse, RuntimeHostBatchExecutionState,
    RuntimeHostBatchMemberFailurePolicy, RuntimeHostBatchMemberReservationDisposition,
    RuntimeHostBatchMemberReservationPolicy, RuntimeHostBatchMemberRetryDisposition,
    RuntimeHostExecutionCancellationContext, RuntimeHostExecutionContractError,
    RuntimeHostExecutionDiagnostic, RuntimeHostExecutionDiagnosticCode,
    RuntimeHostExecutionDiagnosticSeverity, RuntimeHostExecutionRequest,
    RuntimeHostExecutionResponse, RuntimeHostExecutionState,
    ValidatedRuntimeHostBatchExecutionRequest, ValidatedRuntimeHostBatchExecutionResponse,
    ValidatedRuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionResponse,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};

#[test]
fn runtime_host_execution_request_fixture_decodes_and_validates() {
    let request: RuntimeHostExecutionRequest = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture must decode");

    let validated = ValidatedRuntimeHostExecutionRequest::try_from(request)
        .expect("dispatch-selected runtime host request must validate");

    assert_eq!(
        validated.as_ref().contract_version,
        RUNTIME_HOST_EXECUTION_CONTRACT_VERSION
    );
    assert_eq!(
        validated.as_ref().handoff.state,
        SchedulerRuntimeHandoffState::DispatchSelected
    );
    assert_eq!(
        validated
            .as_ref()
            .cancellation_context
            .cancellation_context_id,
        "runtime-host-cancellation.runtime-host.request.001"
    );
    assert_eq!(validated.as_ref().materialized_inputs.len(), 2);
    assert!(validated.as_ref().handoff.dispatch_decision.is_some());
}

#[test]
fn runtime_host_execution_request_requires_cancellation_context() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture must decode as value");
    value
        .as_object_mut()
        .expect("request fixture must be object")
        .remove("cancellation_context");

    let error = serde_json::from_value::<RuntimeHostExecutionRequest>(value)
        .expect_err("runtime host request must explicitly carry cancellation context");

    assert!(
        error
            .to_string()
            .contains("missing field `cancellation_context`"),
        "{error}"
    );
}

#[test]
fn runtime_host_execution_request_rejects_readiness_only_handoff() {
    let mut request: RuntimeHostExecutionRequest = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture must decode");
    request.handoff.state = SchedulerRuntimeHandoffState::ReadinessAdmitted;
    request.handoff.dispatch_decision = None;

    let error = ValidatedRuntimeHostExecutionRequest::try_from(request)
        .expect_err("runtime host execution requires dispatch-selected handoff");

    assert_eq!(
        error,
        RuntimeHostExecutionContractError::InvalidField {
            field: "handoff.state",
            reason: "runtime host execution requires a dispatch-selected scheduler handoff"
        }
    );
}

#[test]
fn runtime_host_execution_request_rejects_path_shaped_fields() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture must decode as value");
    value["model_path"] = json!("/models/juggernaut");

    let error = serde_json::from_value::<RuntimeHostExecutionRequest>(value)
        .expect_err("runtime host request must reject path-shaped fields");

    assert!(
        error.to_string().contains("unknown field `model_path`"),
        "{error}"
    );
}

#[test]
fn runtime_host_execution_request_requires_materialized_inputs_field() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture must decode as value");
    value
        .as_object_mut()
        .expect("request fixture must be object")
        .remove("materialized_inputs");

    let error = serde_json::from_value::<RuntimeHostExecutionRequest>(value)
        .expect_err("runtime host request must explicitly carry materialized inputs");

    assert!(
        error
            .to_string()
            .contains("missing field `materialized_inputs`"),
        "{error}"
    );
}

#[test]
fn runtime_host_execution_request_rejects_path_shaped_input_fields() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture must decode as value");
    value["materialized_inputs"][0]["value"]["path"] = json!("/tmp/input.png");

    let error = serde_json::from_value::<RuntimeHostExecutionRequest>(value)
        .expect_err("runtime host inputs must reject path-shaped fields");

    assert!(error.to_string().contains("string \"path\""), "{error}");
}

#[test]
fn runtime_host_execution_request_rejects_too_many_inputs() {
    let mut request: RuntimeHostExecutionRequest = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture must decode");
    let input = request
        .materialized_inputs
        .first()
        .expect("fixture must contain input")
        .clone();
    request.materialized_inputs = vec![input; 129];

    let error = ValidatedRuntimeHostExecutionRequest::try_from(request)
        .expect_err("runtime host request inputs must be bounded");

    assert_eq!(
        error,
        RuntimeHostExecutionContractError::TooManyInputs {
            actual: 129,
            max: 128
        }
    );
}

#[test]
fn runtime_host_execution_response_fixture_decodes_and_validates() {
    let response: RuntimeHostExecutionResponse = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_accepted.json"
    ))
    .expect("runtime host response fixture must decode");

    let validated = ValidatedRuntimeHostExecutionResponse::try_from(response)
        .expect("accepted runtime host response must validate");

    assert_eq!(
        validated.as_ref().state,
        RuntimeHostExecutionState::Accepted
    );
}

#[test]
fn runtime_host_completed_response_accepts_typed_path_free_outputs() {
    let response: RuntimeHostExecutionResponse = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_completed_outputs.json"
    ))
    .expect("completed runtime host response fixture must decode");

    let validated = ValidatedRuntimeHostExecutionResponse::try_from(response)
        .expect("completed runtime host response with typed outputs must validate");

    assert_eq!(
        validated.as_ref().state,
        RuntimeHostExecutionState::Completed
    );
    assert_eq!(validated.as_ref().outputs.len(), 2);
    assert!(validated.as_ref().terminal_metadata.is_some());
}

#[test]
fn runtime_host_response_rejects_path_shaped_output_fields() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_completed_outputs.json"
    ))
    .expect("completed runtime host response fixture must decode as value");
    value["outputs"][0]["value"]["path"] = json!("/tmp/generated.png");

    let error = serde_json::from_value::<RuntimeHostExecutionResponse>(value)
        .expect_err("runtime host outputs must reject path-shaped fields");

    assert!(error.to_string().contains("string \"path\""), "{error}");
}

#[test]
fn runtime_host_response_rejects_too_many_outputs() {
    let mut response: RuntimeHostExecutionResponse = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_completed_outputs.json"
    ))
    .expect("completed runtime host response fixture must decode");
    let output = response
        .outputs
        .first()
        .expect("fixture must contain output")
        .clone();
    response.outputs = vec![output; 65];

    let error = ValidatedRuntimeHostExecutionResponse::try_from(response)
        .expect_err("runtime host response outputs must be bounded");

    assert_eq!(
        error,
        RuntimeHostExecutionContractError::TooManyOutputs {
            actual: 65,
            max: 64
        }
    );
}

#[test]
fn runtime_host_response_rejects_outputs_without_completed_state() {
    let mut response: RuntimeHostExecutionResponse = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_completed_outputs.json"
    ))
    .expect("completed runtime host response fixture must decode");
    response.state = RuntimeHostExecutionState::Accepted;

    let error = ValidatedRuntimeHostExecutionResponse::try_from(response)
        .expect_err("non-completed runtime host response must not carry outputs");

    assert_eq!(
        error,
        RuntimeHostExecutionContractError::InvalidField {
            field: "outputs",
            reason: "runtime-host outputs are valid only on completed responses"
        }
    );
}

#[test]
fn runtime_host_failed_response_requires_diagnostics() {
    let mut response: RuntimeHostExecutionResponse = serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_accepted.json"
    ))
    .expect("runtime host response fixture must decode");
    response.state = RuntimeHostExecutionState::Failed;
    response.diagnostics.clear();

    let error = ValidatedRuntimeHostExecutionResponse::try_from(response)
        .expect_err("failed runtime host response must explain failure");

    assert_eq!(
        error,
        RuntimeHostExecutionContractError::MissingField {
            field: "diagnostics"
        }
    );
}

#[test]
fn runtime_host_batch_execution_request_accepts_grouped_member_contract() {
    let request = runtime_host_batch_request_fixture();

    let validated = ValidatedRuntimeHostBatchExecutionRequest::try_from(request)
        .expect("runtime host batch request must validate");

    assert_eq!(
        validated.as_ref().contract_version,
        RUNTIME_HOST_EXECUTION_CONTRACT_VERSION
    );
    assert_eq!(
        validated.as_ref().anchor_execution_request_id,
        "runtime-host.request.001"
    );
    assert_eq!(validated.as_ref().members.len(), 2);
    assert!(validated
        .as_ref()
        .members
        .iter()
        .all(|member| member.handoff.state == SchedulerRuntimeHandoffState::DispatchSelected));
}

#[test]
fn runtime_host_batch_execution_request_rejects_empty_members() {
    let mut request = runtime_host_batch_request_fixture();
    request.members.clear();

    let error = ValidatedRuntimeHostBatchExecutionRequest::try_from(request)
        .expect_err("batch request must include members");

    assert_eq!(
        error,
        RuntimeHostExecutionContractError::MissingField { field: "members" }
    );
}

#[test]
fn runtime_host_batch_execution_request_rejects_missing_anchor_member() {
    let mut request = runtime_host_batch_request_fixture();
    request.anchor_execution_request_id = "runtime-host.request.missing".to_string();

    let error = ValidatedRuntimeHostBatchExecutionRequest::try_from(request)
        .expect_err("batch anchor must identify a member");

    assert_eq!(
        error,
        RuntimeHostExecutionContractError::InvalidField {
            field: "anchor_execution_request_id",
            reason: "batch anchor must identify one member execution request"
        }
    );
}

#[test]
fn runtime_host_batch_execution_response_accepts_partial_member_failure_fanout() {
    let response = runtime_host_batch_response_fixture();

    let validated = ValidatedRuntimeHostBatchExecutionResponse::try_from(response)
        .expect("runtime host batch response must validate");

    assert_eq!(
        validated.as_ref().state,
        RuntimeHostBatchExecutionState::PartiallyCompleted
    );
    assert_eq!(validated.as_ref().members.len(), 2);
    assert_eq!(
        validated.as_ref().members[0].reservation_disposition,
        RuntimeHostBatchMemberReservationDisposition::RetainedForRuntimeReuse
    );
    assert_eq!(
        validated.as_ref().members[1].retry_disposition,
        RuntimeHostBatchMemberRetryDisposition::Retryable
    );
}

#[test]
fn runtime_host_batch_execution_response_rejects_outputs_on_failed_member() {
    let mut response = runtime_host_batch_response_fixture();
    let completed_outputs = completed_runtime_host_response_fixture().outputs;
    response.members[1].outputs = completed_outputs;

    let error = ValidatedRuntimeHostBatchExecutionResponse::try_from(response)
        .expect_err("failed batch member must not carry outputs");

    assert_eq!(
        error,
        RuntimeHostExecutionContractError::InvalidField {
            field: "member.outputs",
            reason: "runtime-host batch member outputs are valid only on completed members"
        }
    );
}

fn runtime_host_batch_request_fixture() -> RuntimeHostBatchExecutionRequest {
    let base_request = runtime_host_request_fixture();
    RuntimeHostBatchExecutionRequest {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: "runtime-host.batch.request.001".to_string(),
        anchor_execution_request_id: "runtime-host.request.001".to_string(),
        cancellation_context: RuntimeHostExecutionCancellationContext::workflow_service(
            "runtime-host.batch.request.001",
        ),
        members: vec![
            RuntimeHostBatchExecutionMemberRequest {
                execution_request_id: "runtime-host.request.001".to_string(),
                assignment_id: "assignment.1".to_string(),
                handoff: base_request.handoff.clone(),
                materialized_inputs: base_request.materialized_inputs.clone(),
                timeout_ms: Some(30_000),
                failure_policy: RuntimeHostBatchMemberFailurePolicy::Retryable,
                reservation_policy: RuntimeHostBatchMemberReservationPolicy::RetainForRuntimeReuse,
            },
            RuntimeHostBatchExecutionMemberRequest {
                execution_request_id: "runtime-host.request.002".to_string(),
                assignment_id: "assignment.2".to_string(),
                handoff: base_request.handoff,
                materialized_inputs: base_request.materialized_inputs,
                timeout_ms: Some(30_000),
                failure_policy: RuntimeHostBatchMemberFailurePolicy::Retryable,
                reservation_policy: RuntimeHostBatchMemberReservationPolicy::RetainForRuntimeReuse,
            },
        ],
    }
}

fn runtime_host_batch_response_fixture() -> RuntimeHostBatchExecutionResponse {
    let request = runtime_host_batch_request_fixture();
    let outputs = completed_runtime_host_response_fixture().outputs;
    RuntimeHostBatchExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: request.batch_execution_request_id,
        state: RuntimeHostBatchExecutionState::PartiallyCompleted,
        members: vec![
            RuntimeHostBatchExecutionMemberResponse {
                execution_request_id: request.members[0].execution_request_id.clone(),
                assignment_id: request.members[0].assignment_id.clone(),
                workflow_id: request.members[0].handoff.workflow_id.clone(),
                workflow_run_id: request.members[0].handoff.workflow_run_id.clone(),
                node_id: request.members[0].handoff.node_id.clone(),
                task_id: request.members[0].handoff.task_id.clone(),
                state: RuntimeHostBatchExecutionMemberState::Completed,
                retry_disposition: RuntimeHostBatchMemberRetryDisposition::NotRetryable,
                reservation_disposition:
                    RuntimeHostBatchMemberReservationDisposition::RetainedForRuntimeReuse,
                outputs,
                diagnostics: vec![runtime_host_batch_diagnostic(
                    RuntimeHostExecutionDiagnosticCode::ExecutionCompleted,
                    "batch member completed",
                )],
                terminal_metadata: Some(super::RuntimeHostExecutionTerminalMetadata {
                    completed_at_ms: Some(1_000),
                    attempt: Some(1),
                }),
            },
            RuntimeHostBatchExecutionMemberResponse {
                execution_request_id: request.members[1].execution_request_id.clone(),
                assignment_id: request.members[1].assignment_id.clone(),
                workflow_id: request.members[1].handoff.workflow_id.clone(),
                workflow_run_id: request.members[1].handoff.workflow_run_id.clone(),
                node_id: request.members[1].handoff.node_id.clone(),
                task_id: request.members[1].handoff.task_id.clone(),
                state: RuntimeHostBatchExecutionMemberState::Failed,
                retry_disposition: RuntimeHostBatchMemberRetryDisposition::Retryable,
                reservation_disposition:
                    RuntimeHostBatchMemberReservationDisposition::DeferredToScheduler,
                outputs: Vec::new(),
                diagnostics: vec![runtime_host_batch_diagnostic(
                    RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                    "batch member failed",
                )],
                terminal_metadata: Some(super::RuntimeHostExecutionTerminalMetadata {
                    completed_at_ms: Some(1_001),
                    attempt: Some(1),
                }),
            },
        ],
        diagnostics: vec![runtime_host_batch_diagnostic(
            RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
            "batch partially completed",
        )],
    }
}

fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
    serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture must decode")
}

fn completed_runtime_host_response_fixture() -> RuntimeHostExecutionResponse {
    serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_completed_outputs.json"
    ))
    .expect("runtime host completed response fixture must decode")
}

fn runtime_host_batch_diagnostic(
    code: RuntimeHostExecutionDiagnosticCode,
    message: &str,
) -> RuntimeHostExecutionDiagnostic {
    RuntimeHostExecutionDiagnostic {
        severity: RuntimeHostExecutionDiagnosticSeverity::Error,
        code,
        message: message.to_string(),
        hint: None,
    }
}
