use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_dependency_planning::{
    DependencyEnvironmentReadinessState, DependencyPreflightResult, DependencyReadinessPolicy,
    ValidatedDependencyReadinessRequest,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionPort, RuntimeHostExecutionPortError, RuntimeHostExecutionRequest,
    RuntimeHostExecutionResponse, SchedulerRuntimeHostDispatcher,
};
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerTaskState, SchedulerTaskStateDiagnosticCode,
    SchedulerTaskStateKind, SchedulerWorkflowId, SchedulerWorkflowRunId,
};

use crate::workflow::{
    WorkflowExecutionSessionRunRequest, WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskGraph, WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
};

use super::super::WorkflowExecutionSessionStore;
use super::{
    WorkflowDependencyReadinessLifecycle, WorkflowDependencyReadinessProvider,
    WorkflowDependencyReadinessProviderError,
};
use crate::scheduler::WorkflowSchedulerTaskOrchestrator;

#[test]
fn readiness_lifecycle_builds_request_and_admits_ready_provider_result() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let lifecycle = WorkflowDependencyReadinessLifecycle::new(orchestrator.clone());
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);
    let mut store = initialized_store(&orchestrator, &task_graph);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_graph,
        )
        .expect("initialize active run task state");
    let provider = RecordingReadinessProvider::new(Some(ready_preflight_result()));

    let record = lifecycle
        .resolve_and_admit_active_runtime_task(
            &mut store,
            &provider,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
        )
        .expect("ready provider result should admit runtime task");

    assert_eq!(record.state_version, 2);
    assert_eq!(record.state.kind(), SchedulerTaskStateKind::Ready);
    let request = provider.last_request().expect("provider request");
    assert_eq!(
        request.as_request().planning_request.model_ref,
        task_intent.model_ref
    );
    assert_eq!(
        request.as_request().planning_request.task_id,
        task_intent.task_type
    );
    assert_eq!(
        request
            .as_request()
            .planning_request
            .caller_context
            .run_id
            .as_deref(),
        Some(task_intent.workflow_run_id.as_str())
    );
    assert_eq!(
        request
            .as_request()
            .planning_request
            .scheduler_intent
            .requested_runtime_id,
        task_intent.constraints.requested_runtime_id
    );
}

#[test]
fn readiness_lifecycle_defers_when_provider_has_no_proof() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let lifecycle = WorkflowDependencyReadinessLifecycle::new(orchestrator.clone());
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);
    let mut store = initialized_store(&orchestrator, &task_graph);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_graph,
        )
        .expect("initialize active run task state");
    let provider = RecordingReadinessProvider::new(None);

    let record = lifecycle
        .resolve_and_admit_active_runtime_task(
            &mut store,
            &provider,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
        )
        .expect("missing provider proof should defer runtime task");

    let SchedulerTaskState::PausedDeferred { diagnostics, .. } = record.state else {
        panic!("missing proof should defer the runtime task");
    };
    assert!(diagnostics
        .iter()
        .any(|diagnostic| { diagnostic.code == SchedulerTaskStateDiagnosticCode::TaskDeferred }));
}

#[test]
fn readiness_lifecycle_rejects_mismatched_provider_proof_through_scheduler_policy() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let lifecycle = WorkflowDependencyReadinessLifecycle::new(orchestrator.clone());
    let task_intent = runtime_host_request_fixture().handoff.task_intent;
    let task_graph = task_graph(vec![task_from_intent(task_intent.clone())]);
    let mut store = initialized_store(&orchestrator, &task_graph);
    let session_id = begin_active_run_for_task_graph(&mut store, &task_graph);
    orchestrator
        .initialize_active_run_task_state(
            &mut store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_graph,
        )
        .expect("initialize active run task state");
    let mut stale_proof = ready_preflight_result();
    stale_proof.identity_key.model_ref.model_id = "pumas:other-model".to_string();
    let provider = RecordingReadinessProvider::new(Some(stale_proof));

    let record = lifecycle
        .resolve_and_admit_active_runtime_task(
            &mut store,
            &provider,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
        )
        .expect("mismatched proof should become typed scheduler failure");

    let SchedulerTaskState::TerminalFailed { diagnostics } = record.state else {
        panic!("mismatched proof should terminal-fail the runtime task");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SchedulerTaskStateDiagnosticCode::SchedulerPolicyError
    }));
}

#[derive(Default)]
struct RecordingRuntimeHostPort;

#[async_trait]
impl RuntimeHostExecutionPort for RecordingRuntimeHostPort {
    async fn execute_runtime_host_request(
        &self,
        _request: RuntimeHostExecutionRequest,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        Err(RuntimeHostExecutionPortError::ExecutionFailed {
            message: "runtime host should not be called by readiness lifecycle tests".to_string(),
        })
    }
}

struct RecordingReadinessProvider {
    result: Mutex<Option<DependencyPreflightResult>>,
    last_request: Mutex<Option<ValidatedDependencyReadinessRequest>>,
}

impl RecordingReadinessProvider {
    fn new(result: Option<DependencyPreflightResult>) -> Self {
        Self {
            result: Mutex::new(result),
            last_request: Mutex::new(None),
        }
    }

    fn last_request(&self) -> Option<ValidatedDependencyReadinessRequest> {
        self.last_request
            .lock()
            .expect("provider request lock")
            .clone()
    }
}

impl WorkflowDependencyReadinessProvider for RecordingReadinessProvider {
    fn resolve_dependency_readiness(
        &self,
        request: &ValidatedDependencyReadinessRequest,
    ) -> Result<Option<DependencyPreflightResult>, WorkflowDependencyReadinessProviderError> {
        *self.last_request.lock().expect("provider request lock") = Some(request.clone());
        Ok(self.result.lock().expect("provider result lock").clone())
    }
}

fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
    serde_json::from_str(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture")
}

fn ready_preflight_result() -> DependencyPreflightResult {
    let mut result = runtime_host_request_fixture()
        .handoff
        .readiness_proof
        .preflight_result;
    result.readiness_state = DependencyEnvironmentReadinessState::Ready;
    result
}

fn orchestrator_without_runtime_host_response() -> WorkflowSchedulerTaskOrchestrator {
    WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(Arc::new(
        RecordingRuntimeHostPort,
    )))
}

fn initialized_store(
    _orchestrator: &WorkflowSchedulerTaskOrchestrator,
    _task_graph: &WorkflowSchedulerTaskGraph,
) -> WorkflowExecutionSessionStore {
    WorkflowExecutionSessionStore::new(4, 2)
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
