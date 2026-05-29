use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_scheduler::SchedulerRuntimeHandoffState;

use super::{
    RuntimeHostDispatchError, RuntimeHostExecutionPort, RuntimeHostExecutionPortError,
    SchedulerRuntimeHostDispatcher,
};
use crate::{
    RuntimeHostExecutionContractError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState,
};

#[derive(Default)]
struct RecordingRuntimeHostPort {
    requests: Mutex<Vec<RuntimeHostExecutionRequest>>,
    response: Mutex<Option<RuntimeHostExecutionResponse>>,
}

impl RecordingRuntimeHostPort {
    fn with_response(response: RuntimeHostExecutionResponse) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            response: Mutex::new(Some(response)),
        }
    }

    fn requests(&self) -> Vec<RuntimeHostExecutionRequest> {
        self.requests.lock().expect("request lock").clone()
    }
}

#[async_trait]
impl RuntimeHostExecutionPort for RecordingRuntimeHostPort {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        self.requests.lock().expect("request lock").push(request);
        self.response
            .lock()
            .expect("response lock")
            .clone()
            .ok_or_else(|| RuntimeHostExecutionPortError::ExecutionFailed {
                message: "missing test response".to_string(),
            })
    }
}

#[tokio::test]
async fn dispatcher_passes_dispatch_selected_handoff_to_runtime_host_port() {
    let request = runtime_host_request_fixture();
    let mut response = runtime_host_response_fixture();
    response.execution_request_id = "runtime-host-request-1".to_string();
    let port = Arc::new(RecordingRuntimeHostPort::with_response(response));
    let dispatcher = SchedulerRuntimeHostDispatcher::new(port.clone());

    let validated = dispatcher
        .dispatch(
            "runtime-host-request-1",
            request.handoff,
            request.materialized_inputs.clone(),
        )
        .await
        .expect("dispatch-selected handoff should execute through port");

    assert_eq!(
        validated.as_ref().state,
        RuntimeHostExecutionState::Accepted
    );
    let recorded = port.requests();
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0].handoff.state,
        SchedulerRuntimeHandoffState::DispatchSelected
    );
    assert!(recorded[0].handoff.dispatch_decision.is_some());
    assert_eq!(recorded[0].materialized_inputs.len(), 2);
}

#[tokio::test]
async fn dispatcher_rejects_readiness_only_handoff_before_port_call() {
    let mut request = runtime_host_request_fixture();
    request.handoff.state = SchedulerRuntimeHandoffState::ReadinessAdmitted;
    request.handoff.dispatch_decision = None;
    let port = Arc::new(RecordingRuntimeHostPort::with_response(
        runtime_host_response_fixture(),
    ));
    let dispatcher = SchedulerRuntimeHostDispatcher::new(port.clone());

    let error = dispatcher
        .dispatch(
            "runtime-host-request-1",
            request.handoff,
            request.materialized_inputs,
        )
        .await
        .expect_err("readiness-only handoff must not reach runtime host");

    assert!(matches!(
        error,
        RuntimeHostDispatchError::RequestContract(
            RuntimeHostExecutionContractError::InvalidField {
                field: "handoff.state",
                ..
            }
        )
    ));
    assert!(port.requests().is_empty());
}

#[tokio::test]
async fn dispatcher_rejects_mismatched_runtime_host_response_correlation() {
    let request = runtime_host_request_fixture();
    let mut response = runtime_host_response_fixture();
    response.execution_request_id = "different-request".to_string();
    let port = Arc::new(RecordingRuntimeHostPort::with_response(response));
    let dispatcher = SchedulerRuntimeHostDispatcher::new(port);

    let error = dispatcher
        .dispatch(
            "runtime-host-request-1",
            request.handoff,
            request.materialized_inputs,
        )
        .await
        .expect_err("response must match scheduler request correlation");

    assert!(matches!(
        error,
        RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "execution_request_id",
            ..
        }
    ));
}

fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
    serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture")
}

fn runtime_host_response_fixture() -> RuntimeHostExecutionResponse {
    serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_accepted.json"
    ))
    .expect("runtime host response fixture")
}
