use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_runtime_host_contracts::{
    RuntimeHostDispatchError, RuntimeHostExecutionContractError, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    RuntimeHostExecutionState, SchedulerRuntimeHostDispatcher,
};
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerNodeId, SchedulerRuntimeDeviceConstraints,
    SchedulerRuntimeHandoffState, SchedulerTaskId, SchedulerTaskState,
    SchedulerTaskStateDiagnosticCode, SchedulerTaskStateDiagnosticSeverity, SchedulerWorkflowId,
    SchedulerWorkflowRunId,
};

use crate::workflow::{
    WorkflowExecutionSessionRunRequest, WorkflowSchedulerNonRuntimeTaskTemplate,
    WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskInputBinding, WorkflowSchedulerTaskIntentTemplate,
    WorkflowSchedulerTaskProjectionDiagnostic, WorkflowSchedulerTaskProjectionDiagnosticCode,
    WorkflowSchedulerTaskProjectionDiagnosticSeverity, WorkflowSchedulerTaskResultValue,
    WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
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

#[test]
fn orchestrator_initializes_ready_state_for_schedulable_task() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].task_id.as_str(), task_intent.task_id.as_str());
    assert_eq!(records[0].state_version, 1);
    assert!(matches!(records[0].state, SchedulerTaskState::Ready { .. }));
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
        }),
        non_runtime_task_template: None,
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
fn orchestrator_initializes_ready_non_runtime_state_for_source_task() {
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
        non_runtime_task_template: Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextInput {
            value: "paint a red cube".to_string(),
        }),
        diagnostics: Vec::new(),
    }]);

    let records = orchestrator
        .initial_task_state_records(&task_graph)
        .expect("initial task state records");

    let SchedulerTaskState::Ready { execution_intent } = &records[0].state else {
        panic!("expected ready non-runtime state");
    };
    let task_intent = execution_intent
        .non_runtime_task_intent()
        .expect("non-runtime task intent");
    assert_eq!(task_intent.task_kind.as_str(), "text-input");
    assert_eq!(task_intent.task_id.as_str(), "prompt");
    assert!(execution_intent.runtime_task_intent().is_none());
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
        diagnostics: vec![WorkflowSchedulerTaskProjectionDiagnostic {
            severity: WorkflowSchedulerTaskProjectionDiagnosticSeverity::Error,
            code: WorkflowSchedulerTaskProjectionDiagnosticCode::MissingPumasModelRef,
            node_id: SchedulerNodeId::parse("image-task").expect("node id"),
            port_id: Some("pumas_model_ref".to_string()),
            message: "inference scheduler tasks require canonical pumas_model_ref input"
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
        .hint
        .as_deref()
        .expect("diagnostic hint")
        .contains("pumas_model_ref"));
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
        }),
        non_runtime_task_template: None,
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
    let task_graph = task_graph(vec![text_input_task("prompt", "paint a red cube")]);
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

    let result = orchestrator
        .execute_ready_non_runtime_task(&mut store, &session_id, &workflow_run_id, "prompt")
        .await
        .expect("execute ready non-runtime task");

    assert_eq!(result.task_id, "prompt");
    assert_eq!(
        result.outputs[0].value,
        WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string())
    );
    let (_stored_graph, records) = store
        .active_run_scheduler_task_state(&session_id, &workflow_run_id)
        .expect("active run task state")
        .expect("stored task state");
    assert_eq!(
        records[0].state.kind(),
        pantograph_scheduler::SchedulerTaskStateKind::Completed
    );
    let results = store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].task_id, "prompt");
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
        .execute_ready_non_runtime_task(&mut store, &session_id, &workflow_run_id, &task_id)
        .await
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
        pantograph_scheduler::SchedulerTaskStateKind::Ready
    );
    assert!(store
        .active_run_scheduler_task_results(&session_id, &workflow_run_id)
        .expect("stored task results")
        .is_empty());
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

    let error = orchestrator
        .execute_ready_non_runtime_task(&mut store, &session_id, &workflow_run_id, "text-output")
        .await
        .expect_err("missing materialized input should fail");

    assert!(matches!(
        error,
        WorkflowSchedulerTaskOrchestratorError::NonRuntimeTaskAdapter(_)
    ));
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
        diagnostics: Vec::new(),
    }
}

fn text_input_task(task_id: &str, value: &str) -> WorkflowSchedulerTask {
    WorkflowSchedulerTask {
        workflow_id: scheduler_workflow_id(),
        workflow_run_id: scheduler_workflow_run_id(),
        node_id: SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        node_type: "text-input".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent: None,
        schedulable_intent_template: None,
        non_runtime_task_template: Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextInput {
            value: value.to_string(),
        }),
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

fn scheduler_workflow_id() -> SchedulerWorkflowId {
    SchedulerWorkflowId::parse("workflow.image_generation").expect("workflow id")
}

fn scheduler_workflow_run_id() -> SchedulerWorkflowRunId {
    SchedulerWorkflowRunId::parse("run.001").expect("workflow run id")
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
