use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_scheduler::SchedulerRuntimeHandoffState;

use super::{
    validate_batch_response_matches_request, RuntimeHostBatchExecutionPort,
    RuntimeHostDispatchError, RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, SchedulerRuntimeHostBatchDispatcher,
    SchedulerRuntimeHostDispatcher,
};
use crate::{
    RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionMemberResponse,
    RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionRequest,
    RuntimeHostBatchExecutionResponse, RuntimeHostBatchExecutionState,
    RuntimeHostBatchMemberFailurePolicy, RuntimeHostBatchMemberReservationDisposition,
    RuntimeHostBatchMemberReservationPolicy, RuntimeHostBatchMemberRetryDisposition,
    RuntimeHostExecutionCancellationSnapshot, RuntimeHostExecutionCancellationState,
    RuntimeHostExecutionContractError, RuntimeHostExecutionDiagnostic,
    RuntimeHostExecutionDiagnosticCode, RuntimeHostExecutionDiagnosticSeverity,
    RuntimeHostExecutionRequest, RuntimeHostExecutionResponse, RuntimeHostExecutionState,
    RuntimeHostExecutionTerminalMetadata, RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};

#[derive(Default)]
struct RecordingRuntimeHostPort {
    requests: Mutex<Vec<RuntimeHostExecutionRequest>>,
    cancellation_snapshots: Mutex<Vec<RuntimeHostExecutionCancellationSnapshot>>,
    response: Mutex<Option<RuntimeHostExecutionResponse>>,
}

#[derive(Default)]
struct RecordingRuntimeHostBatchPort {
    requests: Mutex<Vec<RuntimeHostBatchExecutionRequest>>,
    cancellation_snapshots: Mutex<Vec<RuntimeHostExecutionCancellationSnapshot>>,
    response: Mutex<Option<RuntimeHostBatchExecutionResponse>>,
}

impl RecordingRuntimeHostBatchPort {
    fn with_response(response: RuntimeHostBatchExecutionResponse) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            cancellation_snapshots: Mutex::new(Vec::new()),
            response: Mutex::new(Some(response)),
        }
    }

    fn requests(&self) -> Vec<RuntimeHostBatchExecutionRequest> {
        self.requests.lock().expect("request lock").clone()
    }

    fn cancellation_snapshots(&self) -> Vec<RuntimeHostExecutionCancellationSnapshot> {
        self.cancellation_snapshots
            .lock()
            .expect("cancellation snapshot lock")
            .clone()
    }
}

impl RecordingRuntimeHostPort {
    fn with_response(response: RuntimeHostExecutionResponse) -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            cancellation_snapshots: Mutex::new(Vec::new()),
            response: Mutex::new(Some(response)),
        }
    }

    fn requests(&self) -> Vec<RuntimeHostExecutionRequest> {
        self.requests.lock().expect("request lock").clone()
    }

    fn cancellation_snapshots(&self) -> Vec<RuntimeHostExecutionCancellationSnapshot> {
        self.cancellation_snapshots
            .lock()
            .expect("cancellation snapshot lock")
            .clone()
    }
}

#[async_trait]
impl RuntimeHostExecutionPort for RecordingRuntimeHostPort {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        self.cancellation_snapshots
            .lock()
            .expect("cancellation snapshot lock")
            .push(cancellation.snapshot());
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

#[async_trait]
impl RuntimeHostBatchExecutionPort for RecordingRuntimeHostBatchPort {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        self.cancellation_snapshots
            .lock()
            .expect("cancellation snapshot lock")
            .push(cancellation.snapshot());
        self.requests.lock().expect("request lock").push(request);
        self.response
            .lock()
            .expect("response lock")
            .clone()
            .ok_or_else(|| RuntimeHostExecutionPortError::ExecutionFailed {
                message: "missing test batch response".to_string(),
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
    assert_eq!(
        recorded[0].cancellation_context.cancellation_context_id,
        "runtime-host-cancellation.runtime-host-request-1"
    );
    let cancellation_snapshots = port.cancellation_snapshots();
    assert_eq!(cancellation_snapshots.len(), 1);
    assert_eq!(
        cancellation_snapshots[0].cancellation_context_id,
        recorded[0].cancellation_context.cancellation_context_id
    );
    assert_eq!(
        cancellation_snapshots[0].state,
        RuntimeHostExecutionCancellationState::Running
    );
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

#[test]
fn dispatcher_batch_response_correlation_accepts_member_fanout() {
    let request = runtime_host_batch_request_fixture();
    let response = runtime_host_batch_response_fixture(&request);

    validate_batch_response_matches_request(&request, &response)
        .expect("batch response must correlate to request members");
}

#[test]
fn dispatcher_batch_response_correlation_rejects_unknown_member() {
    let request = runtime_host_batch_request_fixture();
    let mut response = runtime_host_batch_response_fixture(&request);
    response.members[1].execution_request_id = "runtime-host.request.unknown".to_string();

    let error = validate_batch_response_matches_request(&request, &response)
        .expect_err("unknown batch member must fail correlation");

    assert!(matches!(
        error,
        RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "members.execution_request_id",
            ..
        }
    ));
}

#[test]
fn dispatcher_batch_response_correlation_rejects_member_workflow_run_mismatch() {
    let request = runtime_host_batch_request_fixture();
    let mut response = runtime_host_batch_response_fixture(&request);
    response.members[0].workflow_run_id = "run.mismatched".parse().expect("workflow run id");

    let error = validate_batch_response_matches_request(&request, &response)
        .expect_err("member workflow run mismatch must fail correlation");

    assert!(matches!(
        error,
        RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "members.workflow_run_id",
            ..
        }
    ));
}

#[tokio::test]
async fn batch_dispatcher_passes_valid_batch_request_to_runtime_host_port() {
    let request = runtime_host_batch_request_fixture();
    let response = runtime_host_batch_response_fixture(&request);
    let port = Arc::new(RecordingRuntimeHostBatchPort::with_response(response));
    let dispatcher = SchedulerRuntimeHostBatchDispatcher::new(port.clone());

    let validated = dispatcher
        .dispatch_batch(request.clone())
        .await
        .expect("valid batch request should reach runtime host batch port");

    assert_eq!(
        validated.as_ref().state,
        RuntimeHostBatchExecutionState::PartiallyCompleted
    );
    let recorded = port.requests();
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0].batch_execution_request_id,
        request.batch_execution_request_id
    );
    assert_eq!(recorded[0].members.len(), 2);
    let cancellation_snapshots = port.cancellation_snapshots();
    assert_eq!(cancellation_snapshots.len(), 1);
    assert_eq!(
        cancellation_snapshots[0].cancellation_context_id,
        request.cancellation_context.cancellation_context_id
    );
    assert_eq!(
        cancellation_snapshots[0].state,
        RuntimeHostExecutionCancellationState::Running
    );
}

#[tokio::test]
async fn batch_dispatcher_rejects_invalid_batch_request_before_port_call() {
    let mut request = runtime_host_batch_request_fixture();
    request.members[0].handoff.state = SchedulerRuntimeHandoffState::ReadinessAdmitted;
    request.members[0].handoff.dispatch_decision = None;
    let response = runtime_host_batch_response_fixture(&runtime_host_batch_request_fixture());
    let port = Arc::new(RecordingRuntimeHostBatchPort::with_response(response));
    let dispatcher = SchedulerRuntimeHostBatchDispatcher::new(port.clone());

    let error = dispatcher
        .dispatch_batch(request)
        .await
        .expect_err("invalid batch request must not reach port");

    assert!(matches!(
        error,
        RuntimeHostDispatchError::RequestContract(
            RuntimeHostExecutionContractError::InvalidField {
                field: "member.handoff.state",
                ..
            }
        )
    ));
    assert!(port.requests().is_empty());
}

#[tokio::test]
async fn batch_dispatcher_rejects_mismatched_batch_response_correlation() {
    let request = runtime_host_batch_request_fixture();
    let mut response = runtime_host_batch_response_fixture(&request);
    response.members[1].assignment_id = "assignment.mismatched".to_string();
    let port = Arc::new(RecordingRuntimeHostBatchPort::with_response(response));
    let dispatcher = SchedulerRuntimeHostBatchDispatcher::new(port);

    let error = dispatcher
        .dispatch_batch(request)
        .await
        .expect_err("batch response must correlate to request members");

    assert!(matches!(
        error,
        RuntimeHostDispatchError::InvalidResponseCorrelation {
            field: "members.assignment_id",
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

fn runtime_host_completed_response_fixture() -> RuntimeHostExecutionResponse {
    serde_json::from_str(include_str!(
        "../tests/fixtures/runtime_host_execution_response_completed_outputs.json"
    ))
    .expect("runtime host completed response fixture")
}

fn runtime_host_batch_request_fixture() -> RuntimeHostBatchExecutionRequest {
    let base_request = runtime_host_request_fixture();
    RuntimeHostBatchExecutionRequest {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: "runtime-host.batch.request.001".to_string(),
        anchor_execution_request_id: "runtime-host.request.001".to_string(),
        cancellation_context: crate::RuntimeHostExecutionCancellationContext::workflow_service(
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

fn runtime_host_batch_response_fixture(
    request: &RuntimeHostBatchExecutionRequest,
) -> RuntimeHostBatchExecutionResponse {
    let outputs = runtime_host_completed_response_fixture().outputs;
    RuntimeHostBatchExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: request.batch_execution_request_id.clone(),
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
                terminal_metadata: Some(RuntimeHostExecutionTerminalMetadata {
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
                terminal_metadata: Some(RuntimeHostExecutionTerminalMetadata {
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
