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
        "runtime_host_execution_tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
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
    assert!(validated.as_ref().handoff.dispatch_decision.is_some());
}

#[test]
fn runtime_host_execution_request_rejects_readiness_only_handoff() {
    let mut request: RuntimeHostExecutionRequest = serde_json::from_str(include_str!(
        "runtime_host_execution_tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
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
        "runtime_host_execution_tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
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
fn runtime_host_execution_response_fixture_decodes_and_validates() {
    let response: RuntimeHostExecutionResponse = serde_json::from_str(include_str!(
        "runtime_host_execution_tests/fixtures/runtime_host_execution_response_accepted.json"
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
fn runtime_host_failed_response_requires_diagnostics() {
    let mut response: RuntimeHostExecutionResponse = serde_json::from_str(include_str!(
        "runtime_host_execution_tests/fixtures/runtime_host_execution_response_accepted.json"
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
