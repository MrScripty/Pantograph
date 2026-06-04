use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_dependency_planning::{
    DependencyEnvironmentReadinessState, DependencyOverrideFingerprint,
    DependencyReadinessDescriptorFingerprint, DependencyReadinessGraphRevision,
    DependencyReadinessPolicy, DependencyReadinessProofEnvelope,
    DependencyReadinessValidationSessionId, DependencyRequirementsId,
};
use pantograph_runtime_host_contracts::{
    ReservationLifecycleApplication, ReservationLifecycleApplicationState,
    ReservationLifecycleEvent, ReservationLifecyclePort, ReservationLifecyclePortError,
    RuntimeHostDispatchError, RuntimeHostExecutionContractError, RuntimeHostExecutionInputValue,
    RuntimeHostExecutionPort, RuntimeHostExecutionPortError, RuntimeHostExecutionRequest,
    RuntimeHostExecutionResponse, SchedulerRuntimeHostDispatcher,
    RESERVATION_LIFECYCLE_CONTRACT_VERSION,
};
use pantograph_scheduler::{
    select_scheduler_dispatch, SchedulableTaskIntent, SchedulerDispatchSelectionRequest,
    SchedulerDispatchSelectionState, SchedulerNodeId, SchedulerRuntimeDeviceConstraints,
    SchedulerRuntimeHandoffState, SchedulerTaskId, SchedulerTaskState,
    SchedulerTaskStateDiagnosticCode, SchedulerTaskStateDiagnosticSeverity, SchedulerTaskStateKind,
    SchedulerWorkflowId, SchedulerWorkflowRunId, ValidatedSchedulerDispatchSelectionRequest,
};
use serde_json::json;

use crate::workflow::{
    WorkflowExecutionSessionRunRequest, WorkflowPortBinding,
    WorkflowSchedulerNonRuntimeTaskTemplate, WorkflowSchedulerSourceInputTemplate,
    WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskInputBinding, WorkflowSchedulerTaskIntentTemplate,
    WorkflowSchedulerTaskProjectionDiagnostic, WorkflowSchedulerTaskProjectionDiagnosticCode,
    WorkflowSchedulerTaskProjectionDiagnosticSeverity, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultOutput, WorkflowSchedulerTaskResultStatus,
    WorkflowSchedulerTaskResultValue, WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
    WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
};

use super::super::WorkflowExecutionSessionStore;
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

#[derive(Default)]
struct AcceptingReservationLifecyclePort;

#[async_trait]
impl ReservationLifecyclePort for AcceptingReservationLifecyclePort {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        Ok(ReservationLifecycleApplication {
            contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
            lifecycle_event_id: event.lifecycle_event_id,
            reservation_lease_id: event.reservation_lease_id,
            state: ReservationLifecycleApplicationState::Applied,
            diagnostics: Vec::new(),
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
        .dispatch_runtime_handoff(
            "workflow-service-runtime-request-1",
            request.handoff,
            request.materialized_inputs.clone(),
        )
        .await
        .expect("dispatch-selected handoff should reach runtime host port");

    assert_eq!(result.status, WorkflowSchedulerTaskResultStatus::Completed);
    assert_eq!(result.outputs.len(), 2);
    let recorded = port.requests();
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0].handoff.state,
        SchedulerRuntimeHandoffState::DispatchSelected
    );
    assert_eq!(recorded[0].materialized_inputs.len(), 2);
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
        .dispatch_runtime_handoff(
            "workflow-service-runtime-request-1",
            request.handoff,
            request.materialized_inputs,
        )
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

#[tokio::test]
async fn orchestrator_selects_scheduler_dispatch_before_runtime_host_port() {
    let selection_request = dispatch_selection_request_fixture();
    let task_intent = selection_request.task_intent.clone();
    let mut response = runtime_host_response_fixture();
    response.execution_request_id = "workflow-service-runtime-request-2".to_string();
    response.workflow_id = task_intent.workflow_id.clone();
    response.workflow_run_id = task_intent.workflow_run_id.clone();
    response.node_id = task_intent.node_id.clone();
    response.task_id = task_intent.task_id.clone();
    let port = Arc::new(RecordingRuntimeHostPort::with_response(response));
    let orchestrator =
        WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(port.clone()))
            .with_reservation_lifecycle_port(Arc::new(AcceptingReservationLifecyclePort));
    let mut task = task_from_intent(task_intent);
    task.dependency_task_ids = vec![SchedulerTaskId::parse("prompt").expect("task id")];
    task.input_bindings = vec![text_binding("prompt", task.task_id.as_str())];
    let materialized_results = vec![text_result(
        "prompt",
        WorkflowSchedulerTaskResultStatus::Completed,
    )];

    let result = orchestrator
        .select_and_dispatch_runtime_task(
            "workflow-service-runtime-request-2",
            &task,
            &materialized_results,
            ValidatedSchedulerDispatchSelectionRequest::try_from(selection_request)
                .expect("selection request fixture must validate"),
        )
        .await
        .expect("selected scheduler dispatch should reach runtime host port");

    assert_eq!(result.status, WorkflowSchedulerTaskResultStatus::Completed);
    let recorded = port.requests();
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0].handoff.state,
        SchedulerRuntimeHandoffState::DispatchSelected
    );
    let dispatch_decision = recorded[0]
        .handoff
        .dispatch_decision
        .as_ref()
        .expect("selected handoff must carry dispatch decision");
    assert_eq!(
        dispatch_decision.selected_runtime_id.as_str(),
        "diffusers-pytorch"
    );
    assert_eq!(
        dispatch_decision.reservation_lease_id.as_str(),
        "reservation.001"
    );
    assert_eq!(recorded[0].materialized_inputs.len(), 1);
    assert_eq!(recorded[0].materialized_inputs[0].port_id, "text");
    assert_eq!(
        recorded[0].materialized_inputs[0].value,
        RuntimeHostExecutionInputValue::String("paint a red cube".to_string())
    );
}

#[tokio::test]
async fn orchestrator_does_not_dispatch_runtime_host_when_scheduler_selects_no_candidate() {
    let mut selection_request = dispatch_selection_request_fixture();
    selection_request.candidates.clear();
    let port = Arc::new(RecordingRuntimeHostPort::with_response(
        runtime_host_response_fixture(),
    ));
    let orchestrator =
        WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(port.clone()))
            .with_reservation_lifecycle_port(Arc::new(AcceptingReservationLifecyclePort));
    let task = task_from_intent(selection_request.task_intent.clone());

    let error = orchestrator
        .select_and_dispatch_runtime_task(
            "workflow-service-runtime-request-3",
            &task,
            &[],
            ValidatedSchedulerDispatchSelectionRequest::try_from(selection_request)
                .expect("selection request without candidates still validates"),
        )
        .await
        .expect_err("no-selection diagnostics must stop before runtime host");

    assert!(matches!(
        error,
        WorkflowSchedulerTaskOrchestratorError::RuntimeDispatchSelectionNoSelection(selection)
            if selection.state == SchedulerDispatchSelectionState::NoSelection
    ));
    assert!(port.requests().is_empty());
}

#[tokio::test]
async fn orchestrator_rejects_missing_runtime_input_before_runtime_host_port() {
    let selection_request = dispatch_selection_request_fixture();
    let mut task = task_from_intent(selection_request.task_intent.clone());
    task.dependency_task_ids = vec![SchedulerTaskId::parse("prompt").expect("task id")];
    task.input_bindings = vec![text_binding("prompt", task.task_id.as_str())];
    let port = Arc::new(RecordingRuntimeHostPort::with_response(
        runtime_host_response_fixture(),
    ));
    let orchestrator =
        WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(port.clone()))
            .with_reservation_lifecycle_port(Arc::new(AcceptingReservationLifecyclePort));

    let error = orchestrator
        .select_and_dispatch_runtime_task(
            "workflow-service-runtime-request-4",
            &task,
            &[],
            ValidatedSchedulerDispatchSelectionRequest::try_from(selection_request)
                .expect("selection request fixture must validate"),
        )
        .await
        .expect_err("missing runtime input must stop before runtime host");

    assert!(matches!(
        error,
        WorkflowSchedulerTaskOrchestratorError::RuntimeHostTaskInputMapping(_)
    ));
    assert!(port.requests().is_empty());
}

#[test]
fn orchestrator_initializes_runtime_task_waiting_for_dependency_readiness() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].task_id.as_str(), task_intent.task_id.as_str());
    assert_eq!(records[0].state_version, 1);
    assert!(matches!(
        records[0].state,
        SchedulerTaskState::WaitingDependencyReadiness { .. }
    ));
}

#[test]
fn orchestrator_admits_runtime_task_after_ready_dependency_proof() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);
    let mut store = WorkflowExecutionSessionStore::new(4, 2);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_graph,
        )
        .expect("initialize active run task state");

    let record = orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
            Some(ready_readiness_proof()),
        )
        .expect("ready dependency proof should admit runtime task");

    assert_eq!(record.state_version, 2);
    assert!(matches!(record.state, SchedulerTaskState::Ready { .. }));
}

#[test]
fn orchestrator_defers_runtime_task_when_dependency_proof_is_missing() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);
    let mut store = WorkflowExecutionSessionStore::new(4, 2);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_graph,
        )
        .expect("initialize active run task state");

    let record = orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
            None,
        )
        .expect("missing dependency proof should defer runtime task");

    let SchedulerTaskState::PausedDeferred { diagnostics, .. } = record.state else {
        panic!("missing proof should leave runtime task deferred");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SchedulerTaskStateDiagnosticCode::TaskDeferred
            && diagnostic.severity == SchedulerTaskStateDiagnosticSeverity::Warning
    }));
}

#[test]
fn orchestrator_retries_deferred_runtime_dependency_readiness() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);
    let mut store = WorkflowExecutionSessionStore::new(4, 2);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_graph,
        )
        .expect("initialize active run task state");
    let deferred = orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
            None,
        )
        .expect("missing dependency proof should defer runtime task");

    let retried = orchestrator
        .retry_deferred_runtime_dependency_readiness(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
        )
        .expect("deferred runtime task should re-enter dependency readiness");

    assert_eq!(
        deferred.state.kind(),
        SchedulerTaskStateKind::PausedDeferred
    );
    assert_eq!(retried.state_version, deferred.state_version + 1);
    let SchedulerTaskState::WaitingDependencyReadiness { execution_intent } = retried.state else {
        panic!("expected waiting dependency readiness");
    };
    assert!(execution_intent.runtime_task_intent().is_some());
}

#[test]
fn orchestrator_retries_retryable_runtime_dependency_readiness_failure() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);
    let mut store = WorkflowExecutionSessionStore::new(4, 2);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_graph,
        )
        .expect("initialize active run task state");
    let mut proof = ready_readiness_proof();
    proof.preflight_result.readiness_state = DependencyEnvironmentReadinessState::Failed;
    let retryable = orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
            Some(proof),
        )
        .expect("failed dependency proof should produce retryable task state");

    let retried = orchestrator
        .retry_deferred_runtime_dependency_readiness(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
        )
        .expect("retryable runtime task should re-enter dependency readiness");

    assert_eq!(
        retryable.state.kind(),
        SchedulerTaskStateKind::RetryableFailed
    );
    assert_eq!(retried.state_version, retryable.state_version + 1);
    assert!(matches!(
        retried.state,
        SchedulerTaskState::WaitingDependencyReadiness { .. }
    ));
}

#[test]
fn orchestrator_defers_non_ready_dependency_proof_without_legacy_bridge() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);
    let mut store = WorkflowExecutionSessionStore::new(4, 2);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_graph,
        )
        .expect("initialize active run task state");
    let mut proof = ready_readiness_proof();
    proof.preflight_result.readiness_state = DependencyEnvironmentReadinessState::Missing;

    let record = orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
            Some(proof),
        )
        .expect("non-ready dependency proof should fail through scheduler policy");

    assert!(matches!(
        record.state,
        SchedulerTaskState::PausedDeferred { .. }
    ));
}

#[test]
fn orchestrator_initializes_dependent_runtime_task_as_awaiting_inputs() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let mut task = task_from_intent(task_intent);
    task.dependency_task_ids = vec![SchedulerTaskId::parse("prompt").expect("task id")];
    task.input_bindings = vec![text_binding("prompt", task.task_id.as_str())];
    let task_graph = task_graph(vec![text_input_task("prompt", "paint a red cube"), task]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");
    let runtime_record = records
        .iter()
        .find(|record| record.task_id.as_str() == "task.image_generation.001")
        .expect("runtime task record");

    assert_eq!(
        runtime_record.state.kind(),
        SchedulerTaskStateKind::AwaitingInputs
    );
}

#[test]
fn orchestrator_advances_dependent_runtime_task_when_inputs_materialize() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_id = task_intent.task_id.as_str().to_string();
    let mut task = task_from_intent(task_intent.clone());
    task.dependency_task_ids = vec![SchedulerTaskId::parse("prompt").expect("task id")];
    task.input_bindings = vec![text_binding("prompt", task.task_id.as_str())];
    let source = text_input_task_for_runtime_intent(&task_intent, "prompt");
    let task_graph = task_graph(vec![source, task]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");
    store
        .record_active_run_scheduler_task_result(
            &session_id,
            &workflow_run_id,
            text_result_for_runtime_intent(
                &task_intent,
                "prompt",
                WorkflowSchedulerTaskResultStatus::Completed,
            ),
        )
        .expect("record prompt result");

    let advanced = orchestrator
        .advance_awaiting_runtime_task_inputs(&mut store, &session_id, &workflow_run_id, &task_id)
        .expect("advance runtime task")
        .expect("runtime task should advance");

    assert_eq!(advanced.state_version, 2);
    let SchedulerTaskState::WaitingDependencyReadiness { execution_intent } = advanced.state else {
        panic!("expected waiting dependency readiness");
    };
    assert!(execution_intent.runtime_task_intent().is_some());
}

#[test]
fn orchestrator_leaves_dependent_runtime_task_blocked_without_materialized_input() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_id = task_intent.task_id.as_str().to_string();
    let mut task = task_from_intent(task_intent.clone());
    task.dependency_task_ids = vec![SchedulerTaskId::parse("prompt").expect("task id")];
    task.input_bindings = vec![text_binding("prompt", task.task_id.as_str())];
    let source = text_input_task_for_runtime_intent(&task_intent, "prompt");
    let task_graph = task_graph(vec![source, task]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(&mut store, &session_id, &workflow_run_id, task_graph)
        .expect("initialize active run task state");

    let advanced = orchestrator
        .advance_awaiting_runtime_task_inputs(&mut store, &session_id, &workflow_run_id, &task_id)
        .expect("advance runtime task");

    assert!(advanced.is_none());
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    let record = records
        .iter()
        .find(|record| record.task_id.as_str() == task_id)
        .expect("runtime task record");
    assert_eq!(record.state.kind(), SchedulerTaskStateKind::AwaitingInputs);
}

#[test]
fn orchestrator_initializes_awaiting_inputs_for_pre_intent_task() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("image-task").expect("node id"),
        task_id: SchedulerTaskId::parse("image-task").expect("task id"),
        node_type: "llm-inference".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
        dependency_task_ids: vec![SchedulerTaskId::parse("model-task").expect("task id")],
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: Some(WorkflowSchedulerTaskIntentTemplate {
            task_type: "image_generation".parse().expect("task type"),
            constraints: SchedulerRuntimeDeviceConstraints::default(),
            trait_settings: Vec::new(),
            dependency_override_patches: Vec::new(),
            estimate_hints: Vec::new(),
            dependency_readiness_source: dependency_readiness_source(),
        }),
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    assert_eq!(records.len(), 1);
    assert!(matches!(
        records[0].state,
        SchedulerTaskState::AwaitingInputs { .. }
    ));
}

#[test]
fn orchestrator_initializes_source_input_state_as_awaiting_inputs() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("prompt").expect("node id"),
        task_id: SchedulerTaskId::parse("prompt").expect("task id"),
        node_type: "text-input".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::SourceInput,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: Some(WorkflowSchedulerSourceInputTemplate::Text {
            port_id: "text".to_string(),
        }),
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    assert!(matches!(
        records[0].state,
        SchedulerTaskState::AwaitingInputs { .. }
    ));
}

#[test]
fn orchestrator_rejects_non_runtime_task_without_typed_template() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("prompt").expect("node id"),
        task_id: SchedulerTaskId::parse("prompt").expect("task id"),
        node_type: "text-input".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    let SchedulerTaskState::Invalid { diagnostics } = &records[0].state else {
        panic!("expected invalid task state");
    };
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::InvalidTask
    );
    assert!(diagnostics[0]
        .message
        .contains("typed non-runtime execution template"));
}

#[test]
fn orchestrator_initializes_awaiting_inputs_for_dependent_non_runtime_task() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("text-output").expect("node id"),
        task_id: SchedulerTaskId::parse("text-output").expect("task id"),
        node_type: "text-output".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
        dependency_task_ids: vec![SchedulerTaskId::parse("prompt").expect("task id")],
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput),
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    assert!(matches!(
        records[0].state,
        SchedulerTaskState::AwaitingInputs { .. }
    ));
}

#[test]
fn orchestrator_initializes_invalid_state_for_unsupported_task_class() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("settings").expect("node id"),
        task_id: SchedulerTaskId::parse("settings").expect("task id"),
        node_type: "expand-settings".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::Unsupported,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    let SchedulerTaskState::Invalid { diagnostics } = &records[0].state else {
        panic!("expected invalid task state");
    };
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::InvalidTask
    );
    assert!(diagnostics[0].message.contains("expand-settings"));
}

#[test]
fn orchestrator_initializes_pumas_materialization_as_awaiting_inputs() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("model").expect("node id"),
        task_id: SchedulerTaskId::parse("model").expect("task id"),
        node_type: "puma-lib".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::PumasMaterialization,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    let SchedulerTaskState::AwaitingInputs { diagnostics } = &records[0].state else {
        panic!("expected awaiting inputs task state");
    };
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::AwaitingInputs
    );
    assert!(diagnostics[0].message.contains("Pumas"));
}

#[test]
fn orchestrator_marks_unhandled_task_classes_terminal_failed() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("model").expect("node id"),
        task_id: SchedulerTaskId::parse("model").expect("task id"),
        node_type: "puma-lib".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::PumasMaterialization,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(&mut store, &session_id, &workflow_run_id, task_graph)
        .expect("initialize active run task state");

    let failed = orchestrator
        .fail_unhandled_task_classes_for_active_run(&mut store, &session_id, &workflow_run_id)
        .expect("fail unhandled class");

    let SchedulerTaskState::TerminalFailed { diagnostics } = &failed[0].state else {
        panic!("expected terminal failed task");
    };
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::SchedulerPolicyError
    );
    assert!(diagnostics[0].message.contains("PumasMaterialization"));
}

#[test]
fn orchestrator_initializes_invalid_state_for_projection_diagnostics() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("image-task").expect("node id"),
        task_id: SchedulerTaskId::parse("image-task").expect("task id"),
        node_type: "llm-inference".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: vec![WorkflowSchedulerTaskProjectionDiagnostic {
            severity: WorkflowSchedulerTaskProjectionDiagnosticSeverity::Error,
            code: WorkflowSchedulerTaskProjectionDiagnosticCode::MissingInferenceDescriptor,
            node_id: SchedulerNodeId::parse("image-task").expect("node id"),
            port_id: None,
            message: "inference scheduler tasks require a current validated inference descriptor"
                .to_string(),
        }],
    }]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    let SchedulerTaskState::Invalid { diagnostics } = &records[0].state else {
        panic!("expected invalid task state");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].severity,
        SchedulerTaskStateDiagnosticSeverity::Error
    );
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::InvalidTask
    );
    assert!(diagnostics[0]
        .message
        .contains("current validated inference descriptor"));
}

#[test]
fn orchestrator_persists_initial_task_state_for_active_run() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("image-task").expect("node id"),
        task_id: SchedulerTaskId::parse("image-task").expect("task id"),
        node_type: "llm-inference".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: Some(WorkflowSchedulerTaskIntentTemplate {
            task_type: "image_generation".parse().expect("task type"),
            constraints: SchedulerRuntimeDeviceConstraints::default(),
            trait_settings: Vec::new(),
            dependency_override_patches: Vec::new(),
            estimate_hints: Vec::new(),
            dependency_readiness_source: dependency_readiness_source(),
        }),
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = store
        .create_session(
            task_graph.workflow_id.as_str().to_string(),
            None,
            None,
            Vec::new(),
            Vec::new(),
            true,
        )
        .expect("create session");
    let queued_run_id = store
        .enqueue_run_with_id(&session_id, &empty_run_request(), workflow_run_id.clone())
        .expect("enqueue run");
    store
        .begin_queued_run(&session_id, &queued_run_id)
        .expect("begin run")
        .expect("dequeued run");

    orchestrator
        .initialize_active_run_task_state(&mut store, &session_id, &workflow_run_id, task_graph)
        .expect("initialize active run task state");

    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    assert_eq!(records.len(), 1);
    assert!(matches!(
        records[0].state,
        SchedulerTaskState::AwaitingInputs { .. }
    ));
}

#[tokio::test]
async fn orchestrator_executes_ready_non_runtime_task_and_persists_completion() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![
        text_input_task("prompt", "paint a red cube"),
        text_output_task(),
    ]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);

    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");
    store
        .record_active_run_scheduler_task_result(
            &session_id,
            &workflow_run_id,
            text_result("prompt", WorkflowSchedulerTaskResultStatus::Completed),
        )
        .expect("record source input result");
    orchestrator
        .advance_awaiting_non_runtime_task_inputs(
            &mut store,
            &session_id,
            &workflow_run_id,
            "text-output",
        )
        .expect("advance text output")
        .expect("text output should become ready");

    let started = orchestrator
        .start_ready_non_runtime_task(&mut store, &session_id, &workflow_run_id, "text-output")
        .expect("start ready non-runtime task");
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    let text_output_record = records
        .iter()
        .find(|record| record.task_id.as_str() == "text-output")
        .expect("text output record");
    assert_eq!(
        text_output_record.state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::Running
    );

    let result = orchestrator
        .execute_started_non_runtime_task(&started)
        .await
        .expect("execute started non-runtime task");
    let completed = orchestrator
        .complete_started_non_runtime_task(
            &mut store,
            &session_id,
            &workflow_run_id,
            &started,
            result.clone(),
        )
        .expect("complete started non-runtime task");
    assert_eq!(
        completed.state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::Completed
    );

    assert_eq!(result.task_id, "text-output");
    assert_eq!(
        result.outputs[0].value,
        WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string())
    );
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    let text_output_record = records
        .iter()
        .find(|record| record.task_id.as_str() == "text-output")
        .expect("text output record");
    assert_eq!(
        text_output_record.state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::Completed
    );
    let results = store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results");
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|result| result.task_id == "prompt"));
    assert!(results.iter().any(|result| result.task_id == "text-output"));
}

#[tokio::test]
async fn orchestrator_rejects_runtime_task_before_non_runtime_adapter() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_id = task_intent.task_id.as_str().to_string();
    let task_graph = task_graph(vec![task_from_intent(task_intent)]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");

    let error = orchestrator
        .start_ready_non_runtime_task(&mut store, &session_id, &workflow_run_id, &task_id)
        .expect_err("runtime task should be rejected");

    let WorkflowSchedulerTaskOrchestratorError::WorkflowService(error) = error else {
        panic!("expected workflow service error");
    };
    assert!(error.message().contains("not a non-runtime"));
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    assert_eq!(
        records[0].state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::WaitingDependencyReadiness
    );
    assert!(store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results")
        .is_empty());
}

#[test]
fn orchestrator_marks_runtime_tasks_terminal_when_dispatch_is_not_wired() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_id = task_intent.task_id.as_str().to_string();
    let task_graph = task_graph(vec![task_from_intent(task_intent)]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");

    let failed = orchestrator
        .fail_runtime_dispatch_not_wired_for_active_run(&mut store, &session_id, &workflow_run_id)
        .expect("runtime dispatch fail closed");

    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].task_id.as_str(), task_id);
    let SchedulerTaskState::TerminalFailed { diagnostics } = &failed[0].state else {
        panic!("expected terminal failed runtime task");
    };
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::SchedulerPolicyError
    );
    assert!(diagnostics[0]
        .message
        .contains("runtime scheduler task dispatch"));
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    assert_eq!(
        records[0].state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::TerminalFailed
    );
    assert!(store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results")
        .is_empty());
}

#[test]
fn orchestrator_persists_started_runtime_task_result() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_id = task_intent.task_id.as_str().to_string();
    let task_graph = task_graph(vec![task_from_intent(task_intent)]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");
    orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            &workflow_run_id,
            &task_id,
            DependencyReadinessPolicy::CheckOnly,
            Some(ready_readiness_proof()),
        )
        .expect("admit runtime task readiness");

    let started = orchestrator
        .start_ready_runtime_task(&mut store, &session_id, &workflow_run_id, &task_id)
        .expect("start ready runtime task");

    assert_eq!(started.task.task_id.as_str(), task_id);
    assert!(started.materialized_results.is_empty());
    let (_stored_graph, running_records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    assert_eq!(
        running_records[0].state.kind(),
        SchedulerTaskStateKind::Running
    );

    let completed = orchestrator
        .complete_started_runtime_task(
            &mut store,
            &session_id,
            &workflow_run_id,
            &started,
            runtime_task_result_fixture(&task_graph.tasks[0]),
        )
        .expect("complete runtime task");

    assert_eq!(completed.state.kind(), SchedulerTaskStateKind::Completed);
    let results = store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].task_id, task_id);
    assert_eq!(
        results[0].status,
        WorkflowSchedulerTaskResultStatus::Completed
    );
}

#[test]
fn orchestrator_rejects_duplicate_runtime_task_attempt_start() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_id = task_intent.task_id.as_str().to_string();
    let task_graph = task_graph(vec![task_from_intent(task_intent)]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");
    orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            &workflow_run_id,
            &task_id,
            DependencyReadinessPolicy::CheckOnly,
            Some(ready_readiness_proof()),
        )
        .expect("admit runtime task readiness");

    let started = orchestrator
        .start_ready_runtime_task(&mut store, &session_id, &workflow_run_id, &task_id)
        .expect("start ready runtime task");
    assert!(started
        .attempt_id
        .as_str()
        .starts_with("scheduler-task-attempt."));

    let error = orchestrator
        .start_ready_runtime_task(&mut store, &session_id, &workflow_run_id, &task_id)
        .expect_err("duplicate runtime task start must be rejected");

    let WorkflowSchedulerTaskOrchestratorError::WorkflowService(error) = error else {
        panic!("expected workflow-service error");
    };
    assert!(!error.message().is_empty());
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    assert_eq!(records[0].state.kind(), SchedulerTaskStateKind::Running);
    assert!(store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results")
        .is_empty());
}

#[test]
fn orchestrator_rejects_stale_runtime_task_completion_without_mutating_results() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_id = task_intent.task_id.as_str().to_string();
    let task_graph = task_graph(vec![task_from_intent(task_intent)]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");
    orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            &workflow_run_id,
            &task_id,
            DependencyReadinessPolicy::CheckOnly,
            Some(ready_readiness_proof()),
        )
        .expect("admit runtime task readiness");
    let started = orchestrator
        .start_ready_runtime_task(&mut store, &session_id, &workflow_run_id, &task_id)
        .expect("start ready runtime task");
    orchestrator
        .complete_started_runtime_task(
            &mut store,
            &session_id,
            &workflow_run_id,
            &started,
            runtime_task_result_fixture(&task_graph.tasks[0]),
        )
        .expect("complete runtime task");

    let error = orchestrator
        .complete_started_runtime_task(
            &mut store,
            &session_id,
            &workflow_run_id,
            &started,
            runtime_task_result_fixture(&task_graph.tasks[0]),
        )
        .expect_err("stale runtime task completion must be rejected");

    let WorkflowSchedulerTaskOrchestratorError::WorkflowService(error) = error else {
        panic!("expected workflow-service error");
    };
    assert!(error
        .message()
        .contains("has no active attempt for completion"));
    let results = store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].task_id, task_id);
    assert_eq!(
        results[0].status,
        WorkflowSchedulerTaskResultStatus::Completed
    );
}

#[test]
fn orchestrator_preserves_dispatch_no_selection_diagnostics_on_started_runtime_task() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let mut selection_request = dispatch_selection_request_fixture();
    selection_request.candidates.clear();
    let task_intent = selection_request.task_intent.clone();
    let task_id = task_intent.task_id.as_str().to_string();
    let task_graph = task_graph(vec![task_from_intent(task_intent)]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(&mut store, &session_id, &workflow_run_id, task_graph)
        .expect("initialize active run task state");
    orchestrator
        .apply_runtime_dependency_readiness_admission(
            &mut store,
            &session_id,
            &workflow_run_id,
            &task_id,
            DependencyReadinessPolicy::CheckOnly,
            Some(selection_request.readiness_proof.clone()),
        )
        .expect("admit runtime task readiness");
    let started = orchestrator
        .start_ready_runtime_task(&mut store, &session_id, &workflow_run_id, &task_id)
        .expect("start ready runtime task");
    let selection = select_scheduler_dispatch(
        ValidatedSchedulerDispatchSelectionRequest::try_from(selection_request)
            .expect("selection request without candidates should validate"),
    )
    .expect("dispatch selection should return no-selection decision")
    .into_inner();

    let failed = orchestrator
        .fail_started_runtime_task_dispatch_selection(
            &mut store,
            &session_id,
            &workflow_run_id,
            &started,
            &selection,
        )
        .expect("persist no-selection diagnostics");

    let SchedulerTaskState::TerminalFailed { diagnostics } = failed.state else {
        panic!("expected terminal failed runtime task");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::SchedulerPolicyError
    );
    assert!(diagnostics[0].message.contains("NoCandidates"));
    assert!(diagnostics[0].message.contains("No scheduler dispatch"));
}

#[tokio::test]
async fn orchestrator_marks_non_runtime_adapter_failure_terminal_without_result() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("text-output").expect("node id"),
        task_id: SchedulerTaskId::parse("text-output").expect("task id"),
        node_type: "text-output".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
        dependency_task_ids: Vec::new(),
        input_bindings: vec![text_binding("prompt", "text-output")],
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput),
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");

    let started = orchestrator
        .start_ready_non_runtime_task(&mut store, &session_id, &workflow_run_id, "text-output")
        .expect("start ready non-runtime task");
    let error = orchestrator
        .execute_started_non_runtime_task(&started)
        .await
        .expect_err("missing materialized input should fail");

    let WorkflowSchedulerTaskOrchestratorError::NonRuntimeTaskAdapter(adapter_error) = error else {
        panic!("expected non-runtime adapter error");
    };
    orchestrator
        .fail_started_non_runtime_task(
            &mut store,
            &session_id,
            &workflow_run_id,
            &started,
            &adapter_error,
        )
        .expect("fail started non-runtime task");
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    let SchedulerTaskState::TerminalFailed { diagnostics } = &records[0].state else {
        panic!("expected terminal failed task state");
    };
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::TerminalFailure
    );
    assert!(store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results")
        .is_empty());
}

#[test]
fn orchestrator_advances_dependent_non_runtime_task_when_inputs_materialize() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![
        text_input_task("prompt", "paint a red cube"),
        text_output_task(),
    ]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");
    store
        .record_active_run_scheduler_task_result(
            &session_id,
            &workflow_run_id,
            text_result("prompt", WorkflowSchedulerTaskResultStatus::Completed),
        )
        .expect("record prompt result");

    let advanced = orchestrator
        .advance_awaiting_non_runtime_task_inputs(
            &mut store,
            &session_id,
            &workflow_run_id,
            "text-output",
        )
        .expect("advance dependent task")
        .expect("task advanced");

    assert_eq!(
        advanced.state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::Ready
    );
    let SchedulerTaskState::Ready { execution_intent } = advanced.state else {
        panic!("expected ready task state");
    };
    assert!(execution_intent.non_runtime_task_intent().is_some());
}

#[test]
fn orchestrator_materializes_external_source_input_through_task_state() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![
        text_input_task("prompt", "paint a red cube"),
        text_output_task(),
    ]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");

    let completed = orchestrator
        .materialize_external_inputs_for_active_run(
            &mut store,
            &session_id,
            &workflow_run_id,
            &[WorkflowPortBinding {
                node_id: "prompt".to_string(),
                port_id: "text".to_string(),
                value: json!("paint a red cube"),
            }],
        )
        .expect("materialize source input");

    assert_eq!(completed.len(), 1);
    assert_eq!(
        completed[0].state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::Completed
    );
    let SchedulerTaskState::Completed { execution_intent } = &completed[0].state else {
        panic!("expected completed source-input state");
    };
    assert!(execution_intent.source_input_task_intent().is_some());
    assert!(execution_intent.runtime_task_intent().is_none());
    assert!(execution_intent.non_runtime_task_intent().is_none());
    let stored_results = store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results");
    assert_eq!(stored_results.len(), 1);
    assert_eq!(stored_results[0].task_id, "prompt");
    assert_eq!(
        stored_results[0].outputs[0].value,
        WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string())
    );
}

#[test]
fn orchestrator_rejects_invalid_external_source_input_without_partial_store() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![
        text_input_task("prompt", "paint a red cube"),
        text_output_task(),
    ]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(&mut store, &session_id, &workflow_run_id, task_graph)
        .expect("initialize active run task state");

    let error = orchestrator
        .materialize_external_inputs_for_active_run(
            &mut store,
            &session_id,
            &workflow_run_id,
            &[WorkflowPortBinding {
                node_id: "prompt".to_string(),
                port_id: "text".to_string(),
                value: json!(true),
            }],
        )
        .expect_err("wrong source input type should fail");

    assert!(matches!(
        error,
        WorkflowSchedulerTaskOrchestratorError::ExternalInputMaterialization(_)
    ));
    assert!(store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results")
        .is_empty());
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    let prompt_record = records
        .iter()
        .find(|record| record.task_id.as_str() == "prompt")
        .expect("prompt record");
    assert_eq!(
        prompt_record.state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::AwaitingInputs
    );
}

#[test]
fn orchestrator_leaves_dependent_non_runtime_task_blocked_without_materialized_input() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![
        text_input_task("prompt", "paint a red cube"),
        text_output_task(),
    ]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");

    let advanced = orchestrator
        .advance_awaiting_non_runtime_task_inputs(
            &mut store,
            &session_id,
            &workflow_run_id,
            "text-output",
        )
        .expect("advance dependent task");

    assert!(advanced.is_none());
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    let record = records
        .iter()
        .find(|record| record.task_id.as_str() == "text-output")
        .expect("text output record");
    assert_eq!(
        record.state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::AwaitingInputs
    );
}

#[test]
fn orchestrator_marks_dependent_non_runtime_task_invalid_for_wrong_input_type() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_graph = task_graph(vec![
        text_input_task("prompt", "paint a red cube"),
        text_output_task(),
    ]);
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let mut store = WorkflowExecutionSessionStore::new(1, 1);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            &workflow_run_id,
            task_graph.clone(),
        )
        .expect("initialize active run task state");
    store
        .record_active_run_scheduler_task_result(
            &session_id,
            &workflow_run_id,
            bool_result("prompt", WorkflowSchedulerTaskResultStatus::Completed),
        )
        .expect("record prompt result");

    let advanced = orchestrator
        .advance_awaiting_non_runtime_task_inputs(
            &mut store,
            &session_id,
            &workflow_run_id,
            "text-output",
        )
        .expect("advance dependent task")
        .expect("task advanced");

    let SchedulerTaskState::Invalid { diagnostics } = advanced.state else {
        panic!("expected invalid task state");
    };
    assert_eq!(
        diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::InvalidTask
    );
}

fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
    serde_json::from_str(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture")
}

fn ready_readiness_proof() -> DependencyReadinessProofEnvelope {
    runtime_host_request_fixture().handoff.readiness_proof
}

fn runtime_host_response_fixture() -> RuntimeHostExecutionResponse {
    serde_json::from_str(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_response_completed_outputs.json"
    ))
    .expect("runtime host response fixture")
}

fn dispatch_selection_request_fixture() -> SchedulerDispatchSelectionRequest {
    serde_json::from_str(include_str!(
        "../../../pantograph-scheduler/tests/fixtures/dispatch_selection_request_valid.json"
    ))
    .expect("dispatch-selection request fixture")
}

fn orchestrator_without_runtime_host_response() -> WorkflowSchedulerTaskOrchestrator {
    WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(Arc::new(
        RecordingRuntimeHostPort::default(),
    )))
}

fn task_graph(tasks: Vec<WorkflowSchedulerTask>) -> WorkflowSchedulerTaskGraph {
    let workflow_id = tasks
        .first()
        .map(|task| task.workflow_id.clone())
        .unwrap_or_else(scheduler_workflow_id);
    let workflow_run_id = tasks
        .first()
        .map(|task| task.workflow_run_id.clone())
        .unwrap_or_else(scheduler_workflow_run_id);
    WorkflowSchedulerTaskGraph {
        schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        workflow_id,
        workflow_run_id,
        tasks,
    }
}

fn task_from_intent(task_intent: SchedulableTaskIntent) -> WorkflowSchedulerTask {
    WorkflowSchedulerTask {
        workflow_id: task_intent.workflow_id.clone(),
        workflow_run_id: task_intent.workflow_run_id.clone(),
        node_id: task_intent.node_id.clone(),
        task_id: task_intent.task_id.clone(),
        node_type: "llm-inference".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: Some(task_intent),
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }
}

fn runtime_task_result_fixture(task: &WorkflowSchedulerTask) -> WorkflowSchedulerTaskResult {
    WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: task.workflow_id.as_str().to_string(),
        workflow_run_id: task.workflow_run_id.as_str().to_string(),
        node_id: task.node_id.as_str().to_string(),
        task_id: task.task_id.as_str().to_string(),
        status: WorkflowSchedulerTaskResultStatus::Completed,
        outputs: vec![WorkflowSchedulerTaskResultOutput {
            port_id: "image".to_string(),
            value: WorkflowSchedulerTaskResultValue::DiagnosticOnly,
        }],
        diagnostics: Vec::new(),
        terminal_metadata: None,
    }
}

fn text_input_task(task_id: &str, _value: &str) -> WorkflowSchedulerTask {
    WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        node_type: "text-input".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::SourceInput,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: None,
        source_input_task_template: Some(WorkflowSchedulerSourceInputTemplate::Text {
            port_id: "text".to_string(),
        }),
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }
}

fn text_input_task_for_runtime_intent(
    task_intent: &SchedulableTaskIntent,
    task_id: &str,
) -> WorkflowSchedulerTask {
    let mut task = text_input_task(task_id, "paint a red cube");
    task.workflow_id = task_intent.workflow_id.clone();
    task.workflow_run_id = task_intent.workflow_run_id.clone();
    task
}

fn text_output_task() -> WorkflowSchedulerTask {
    WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse("text-output").expect("node id"),
        task_id: SchedulerTaskId::parse("text-output").expect("task id"),
        node_type: "text-output".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
        dependency_task_ids: vec![SchedulerTaskId::parse("prompt").expect("task id")],
        input_bindings: vec![text_binding("prompt", "text-output")],
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput),
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        diagnostics: Vec::new(),
    }
}

fn text_binding(source_task_id: &str, _target_task_id: &str) -> WorkflowSchedulerTaskInputBinding {
    WorkflowSchedulerTaskInputBinding {
        source_node_id: SchedulerNodeId::parse(source_task_id).expect("source node id"),
        source_task_id: SchedulerTaskId::parse(source_task_id).expect("source task id"),
        source_port_id: "text".to_string(),
        target_port_id: "text".to_string(),
    }
}

fn begin_active_run_for_task_graph(
    store: &mut WorkflowExecutionSessionStore,
    task_graph: &WorkflowSchedulerTaskGraph,
) -> String {
    let session_id = store
        .create_session(
            task_graph.workflow_id.as_str().to_string(),
            None,
            None,
            Vec::new(),
            Vec::new(),
            true,
        )
        .expect("create session");
    let workflow_run_id = task_graph.workflow_run_id.as_str().to_string();
    let queued_run_id = store
        .enqueue_run_with_id(&session_id, &empty_run_request(), workflow_run_id)
        .expect("enqueue run");
    store
        .begin_queued_run(&session_id, &queued_run_id)
        .expect("begin run")
        .expect("dequeued run");
    session_id
}

fn text_result(
    task_id: &str,
    status: WorkflowSchedulerTaskResultStatus,
) -> WorkflowSchedulerTaskResult {
    task_result(
        task_id,
        status,
        WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string()),
    )
}

fn text_result_for_runtime_intent(
    task_intent: &SchedulableTaskIntent,
    task_id: &str,
    status: WorkflowSchedulerTaskResultStatus,
) -> WorkflowSchedulerTaskResult {
    let mut result = text_result(task_id, status);
    result.workflow_id = task_intent.workflow_id.as_str().to_string();
    result.workflow_run_id = task_intent.workflow_run_id.as_str().to_string();
    result
}

fn bool_result(
    task_id: &str,
    status: WorkflowSchedulerTaskResultStatus,
) -> WorkflowSchedulerTaskResult {
    task_result(
        task_id,
        status,
        WorkflowSchedulerTaskResultValue::Bool(true),
    )
}

fn task_result(
    task_id: &str,
    status: WorkflowSchedulerTaskResultStatus,
    value: WorkflowSchedulerTaskResultValue,
) -> WorkflowSchedulerTaskResult {
    WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: scheduler_workflow_id().as_str().to_string(),
        workflow_run_id: scheduler_workflow_run_id().as_str().to_string(),
        node_id: task_id.to_string(),
        task_id: task_id.to_string(),
        status,
        outputs: vec![WorkflowSchedulerTaskResultOutput {
            port_id: "text".to_string(),
            value,
        }],
        diagnostics: Vec::new(),
        terminal_metadata: None,
    }
}

fn scheduler_workflow_id() -> SchedulerWorkflowId {
    SchedulerWorkflowId::parse("workflow.image_generation").expect("workflow id")
}

fn scheduler_workflow_run_id() -> SchedulerWorkflowRunId {
    SchedulerWorkflowRunId::parse("run.001").expect("workflow run id")
}

fn dependency_readiness_source() -> crate::workflow::WorkflowSchedulerDependencyReadinessSource {
    crate::workflow::WorkflowSchedulerDependencyReadinessSource {
        graph_revision: DependencyReadinessGraphRevision::parse("graph.revision.001")
            .expect("graph revision"),
        validation_session_id: Some(
            DependencyReadinessValidationSessionId::parse("validation.session.001")
                .expect("validation session"),
        ),
        validation_snapshot_id: None,
        descriptor_fingerprint: DependencyReadinessDescriptorFingerprint::parse(
            "descriptor.fingerprint.001",
        )
        .expect("descriptor fingerprint"),
        dependency_requirements_id: DependencyRequirementsId::parse(
            "dependency-requirements-blake3:test",
        )
        .expect("dependency requirements id"),
        selected_binding_ids: Vec::new(),
        dependency_override_fingerprint: DependencyOverrideFingerprint::parse(
            "dependency-overrides-blake3:test",
        )
        .expect("dependency override fingerprint"),
    }
}

fn empty_run_request() -> WorkflowExecutionSessionRunRequest {
    WorkflowExecutionSessionRunRequest {
        session_id: "ignored".to_string(),
        workflow_semantic_version: "0.1.0".to_string(),
        inputs: Vec::new(),
        output_targets: None,
        override_selection: None,
        timeout_ms: None,
        priority: None,
    }
}
