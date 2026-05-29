use pantograph_scheduler::SchedulerRuntimeHandoffState;
use serde_json::json;

use super::{
    RuntimeHostExecutionContractError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, ValidatedRuntimeHostExecutionRequest,
    ValidatedRuntimeHostExecutionResponse, RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
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
    assert_eq!(validated.as_ref().materialized_inputs.len(), 2);
    assert!(validated.as_ref().handoff.dispatch_decision.is_some());
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
