use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_runtime_host_contracts::{
    RuntimeHostDispatchError, RuntimeHostExecutionContractError, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, SchedulerRuntimeHostDispatcher,
};
use pantograph_scheduler::SchedulerRuntimeHandoffState;

use super::{WorkflowSchedulerTaskOrchestrator, WorkflowSchedulerTaskOrchestratorError};

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
async fn orchestrator_dispatches_runtime_task_through_shared_runtime_host_port() {
    let request = runtime_host_request_fixture();
    let mut response = runtime_host_response_fixture();
    response.execution_request_id = "workflow-service-runtime-request-1".to_string();
    let port = Arc::new(RecordingRuntimeHostPort::with_response(response));
    let orchestrator =
        WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(port.clone()));

    let result = orchestrator
        .dispatch_runtime_handoff("workflow-service-runtime-request-1", request.handoff)
        .await
        .expect("dispatch-selected handoff should reach runtime host port");

    assert_eq!(result.as_ref().state, RuntimeHostExecutionState::Accepted);
    let recorded = port.requests();
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0].handoff.state,
        SchedulerRuntimeHandoffState::DispatchSelected
    );
}

#[tokio::test]
async fn orchestrator_rejects_readiness_only_handoff_before_runtime_host_port() {
    let mut request = runtime_host_request_fixture();
    request.handoff.state = SchedulerRuntimeHandoffState::ReadinessAdmitted;
    request.handoff.dispatch_decision = None;
    let port = Arc::new(RecordingRuntimeHostPort::with_response(
        runtime_host_response_fixture(),
    ));
    let orchestrator =
        WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(port.clone()));

    let error = orchestrator
        .dispatch_runtime_handoff("workflow-service-runtime-request-1", request.handoff)
        .await
        .expect_err("readiness-only handoff must fail before runtime host");

    assert!(matches!(
        error,
        WorkflowSchedulerTaskOrchestratorError::RuntimeHostDispatch(
            RuntimeHostDispatchError::RequestContract(
                RuntimeHostExecutionContractError::InvalidField {
                    field: "handoff.state",
                    ..
                }
            )
        )
    ));
    assert!(port.requests().is_empty());
}

fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
    serde_json::from_str(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture")
}

fn runtime_host_response_fixture() -> RuntimeHostExecutionResponse {
    serde_json::from_str(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_response_accepted.json"
    ))
    .expect("runtime host response fixture")
}
