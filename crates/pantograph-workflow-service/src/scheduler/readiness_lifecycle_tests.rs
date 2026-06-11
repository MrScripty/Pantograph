use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_dependency_environment_service::{
    DependencyEnvironmentService, NotImplementedDependencyEnvironmentProvider,
};
use pantograph_dependency_planning::{
    produce_dependency_requirements_proof, DependencyEnvironmentReadinessState,
    DependencyNodeTypeId, DependencyPlanningCallerContext, DependencyPlanningRequest,
    DependencyPreflightResult, DependencyReadinessDescriptorFingerprint,
    DependencyReadinessGraphRevision, DependencyReadinessPolicy,
    DependencyReadinessValidationSessionId, DependencyTraitIntent, DependencyTraitIntentId,
    DependencyTraitIntentValue, SchedulerIntent, ValidatedDependencyPlanningRequest,
    ValidatedDependencyReadinessRequestEnvelope,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    SchedulerRuntimeHostDispatcher,
};
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerTaskState, SchedulerTaskStateDiagnosticCode,
    SchedulerTaskStateKind, SchedulerWorkflowId, SchedulerWorkflowRunId,
};

use crate::workflow::{
    WorkflowExecutionSessionRunRequest, WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskGraph, WorkflowSchedulerTaskIntentTemplate,
    WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
};

use super::super::WorkflowExecutionSessionStore;
use super::{
    WorkflowDependencyReadinessLifecycle, WorkflowDependencyReadinessProvider,
    WorkflowDependencyReadinessProviderError,
};
use crate::scheduler::lifecycle::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
    WorkflowSchedulerLifecycleComponentState, WorkflowSchedulerLifecycleOwnerId,
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
    let provider = RecordingReadinessProvider::new(Some(ready_preflight_result(&task_intent)));

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
        request
            .as_envelope()
            .readiness_request
            .planning_request
            .model_ref,
        task_intent.model_ref
    );
    assert_eq!(
        request
            .as_envelope()
            .readiness_request
            .planning_request
            .task_id,
        task_intent.task_type
    );
    assert_eq!(
        request
            .as_envelope()
            .readiness_request
            .planning_request
            .caller_context
            .run_id
            .as_deref(),
        Some(task_intent.workflow_run_id.as_str())
    );
    assert_eq!(
        request
            .as_envelope()
            .readiness_request
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
fn readiness_lifecycle_rejects_mismatched_provider_proof_before_scheduler_policy() {
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
    let mut stale_proof = ready_preflight_result(&task_intent);
    stale_proof.identity_key.model_ref.model_id = "pumas:other-model".to_string();
    let provider = RecordingReadinessProvider::new(Some(stale_proof));

    let error = lifecycle
        .resolve_and_admit_active_runtime_task(
            &mut store,
            &provider,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
        )
        .expect_err("mismatched proof should fail envelope validation");

    assert!(error.to_string().contains("dependency planning"));
}

#[test]
fn readiness_lifecycle_uses_dependency_environment_service_provider() {
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
    let provider = DependencyEnvironmentService::new(NotImplementedDependencyEnvironmentProvider);

    let record = lifecycle
        .resolve_and_admit_active_runtime_task(
            &mut store,
            &provider,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
        )
        .expect("not-implemented dependency provider should produce typed terminal state");

    let SchedulerTaskState::TerminalFailed { diagnostics } = record.state else {
        panic!("not-implemented dependency provider should terminal-fail the runtime task");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == SchedulerTaskStateDiagnosticCode::TerminalFailure
    }));
}

#[test]
fn readiness_lifecycle_marks_dependency_readiness_action_during_provider_call() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let scheduler_lifecycle = WorkflowSchedulerLifecycleComponentRegistryHandle::new(
        WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.readiness-lifecycle.test")
            .expect("scheduler lifecycle owner id"),
    );
    let lifecycle = WorkflowDependencyReadinessLifecycle::new_with_scheduler_lifecycle(
        orchestrator.clone(),
        scheduler_lifecycle.clone(),
    );
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
    let provider = LifecycleAssertingReadinessProvider::new(
        Some(ready_preflight_result(&task_intent)),
        scheduler_lifecycle.clone(),
    );

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

    assert_eq!(record.state.kind(), SchedulerTaskStateKind::Ready);
    assert_eq!(
        scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::DependencyReadinessAction)
            .expect("dependency readiness component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );
    assert!(provider.called());
    assert_eq!(
        lifecycle
            .dependency_readiness_lifecycle_component()
            .expect("dependency readiness lifecycle component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );
}

#[test]
fn readiness_lifecycle_marks_dependency_readiness_action_during_seed_provider_call() {
    let orchestrator = orchestrator_without_runtime_host_response();
    let scheduler_lifecycle = WorkflowSchedulerLifecycleComponentRegistryHandle::new(
        WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.readiness-seed.test")
            .expect("scheduler lifecycle owner id"),
    );
    let lifecycle = WorkflowDependencyReadinessLifecycle::new_with_scheduler_lifecycle(
        orchestrator.clone(),
        scheduler_lifecycle.clone(),
    );
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
    let request = lifecycle
        .readiness_request_for_active_runtime_task(
            &store,
            &session_id,
            task_intent.workflow_run_id.as_str(),
            task_intent.task_id.as_str(),
            DependencyReadinessPolicy::CheckOnly,
        )
        .expect("readiness request");
    let provider = SeedLifecycleAssertingReadinessProvider::new(scheduler_lifecycle.clone());

    let seed_result = lifecycle
        .resolve_dependency_requirements_seed(&provider, &request)
        .expect("seed provider should resolve");

    assert!(seed_result.is_none());
    assert!(provider.called());
    assert_eq!(
        scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::DependencyReadinessAction)
            .expect("dependency readiness component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );
}

#[derive(Default)]
struct RecordingRuntimeHostPort;

#[async_trait]
impl RuntimeHostExecutionPort for RecordingRuntimeHostPort {
    async fn execute_runtime_host_request(
        &self,
        _request: RuntimeHostExecutionRequest,
        _cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        Err(RuntimeHostExecutionPortError::ExecutionFailed {
            message: "runtime host should not be called by readiness lifecycle tests".to_string(),
        })
    }
}

struct RecordingReadinessProvider {
    result: Mutex<Option<DependencyPreflightResult>>,
    last_request: Mutex<Option<ValidatedDependencyReadinessRequestEnvelope>>,
}

impl RecordingReadinessProvider {
    fn new(result: Option<DependencyPreflightResult>) -> Self {
        Self {
            result: Mutex::new(result),
            last_request: Mutex::new(None),
        }
    }

    fn last_request(&self) -> Option<ValidatedDependencyReadinessRequestEnvelope> {
        self.last_request
            .lock()
            .expect("provider request lock")
            .clone()
    }
}

impl WorkflowDependencyReadinessProvider for RecordingReadinessProvider {
    fn resolve_dependency_readiness(
        &self,
        request: &ValidatedDependencyReadinessRequestEnvelope,
    ) -> Result<Option<DependencyPreflightResult>, WorkflowDependencyReadinessProviderError> {
        *self.last_request.lock().expect("provider request lock") = Some(request.clone());
        Ok(self.result.lock().expect("provider result lock").clone())
    }
}

struct LifecycleAssertingReadinessProvider {
    result: Mutex<Option<DependencyPreflightResult>>,
    scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    called: Mutex<bool>,
}

impl LifecycleAssertingReadinessProvider {
    fn new(
        result: Option<DependencyPreflightResult>,
        scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    ) -> Self {
        Self {
            result: Mutex::new(result),
            scheduler_lifecycle,
            called: Mutex::new(false),
        }
    }

    fn called(&self) -> bool {
        *self.called.lock().expect("called lock")
    }
}

impl WorkflowDependencyReadinessProvider for LifecycleAssertingReadinessProvider {
    fn resolve_dependency_readiness(
        &self,
        _request: &ValidatedDependencyReadinessRequestEnvelope,
    ) -> Result<Option<DependencyPreflightResult>, WorkflowDependencyReadinessProviderError> {
        *self.called.lock().expect("called lock") = true;
        assert_eq!(
            self.scheduler_lifecycle
                .component(WorkflowSchedulerLifecycleComponentKind::DependencyReadinessAction)
                .expect("dependency readiness component")
                .state,
            WorkflowSchedulerLifecycleComponentState::Running
        );
        Ok(self.result.lock().expect("provider result lock").clone())
    }
}

struct SeedLifecycleAssertingReadinessProvider {
    scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    called: Mutex<bool>,
}

impl SeedLifecycleAssertingReadinessProvider {
    fn new(scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle) -> Self {
        Self {
            scheduler_lifecycle,
            called: Mutex::new(false),
        }
    }

    fn called(&self) -> bool {
        *self.called.lock().expect("called lock")
    }
}

impl WorkflowDependencyReadinessProvider for SeedLifecycleAssertingReadinessProvider {
    fn resolve_dependency_requirements_seed(
        &self,
        _request: &ValidatedDependencyReadinessRequestEnvelope,
    ) -> Result<
        Option<pantograph_dependency_planning::ValidatedDependencyEnvironmentResult>,
        WorkflowDependencyReadinessProviderError,
    > {
        *self.called.lock().expect("called lock") = true;
        assert_eq!(
            self.scheduler_lifecycle
                .component(WorkflowSchedulerLifecycleComponentKind::DependencyReadinessAction)
                .expect("dependency readiness component")
                .state,
            WorkflowSchedulerLifecycleComponentState::Running
        );
        Ok(None)
    }

    fn resolve_dependency_readiness(
        &self,
        _request: &ValidatedDependencyReadinessRequestEnvelope,
    ) -> Result<Option<DependencyPreflightResult>, WorkflowDependencyReadinessProviderError> {
        panic!("seed lifecycle provider should not resolve readiness proof")
    }
}

fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
    serde_json::from_str(include_str!(
        "../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host request fixture")
}

fn ready_preflight_result(task_intent: &SchedulableTaskIntent) -> DependencyPreflightResult {
    let planning_request = dependency_planning_request_for_intent(task_intent);
    let validated_request = ValidatedDependencyPlanningRequest::try_from(planning_request.clone())
        .expect("validated dependency planning request");
    let requirements_proof = produce_dependency_requirements_proof(&validated_request, None)
        .expect("dependency requirements proof");
    let mut result = runtime_host_request_fixture()
        .handoff
        .readiness_proof
        .preflight_result;
    result.identity_key = requirements_proof.identity_key;
    result.dependency_requirements_id = Some(requirements_proof.dependency_requirements_id);
    result.readiness_state = DependencyEnvironmentReadinessState::Ready;
    result
}

fn dependency_planning_request_for_intent(
    task_intent: &SchedulableTaskIntent,
) -> DependencyPlanningRequest {
    DependencyPlanningRequest {
        model_ref: task_intent.model_ref.clone(),
        task_id: task_intent.task_type.clone(),
        task_type: Some(task_intent.task_type.clone()),
        expected_artifact_kind: None,
        scheduler_intent: SchedulerIntent {
            requested_runtime_id: task_intent.constraints.requested_runtime_id.clone(),
            requested_device_id: task_intent.constraints.requested_device_id.clone(),
        },
        platform_context: None,
        selected_binding_ids: Vec::new(),
        dependency_override_patches: task_intent.dependency_override_patches.clone(),
        trait_intents: task_intent
            .trait_settings
            .iter()
            .map(|setting| DependencyTraitIntent {
                trait_id: DependencyTraitIntentId::parse(setting.trait_id.as_str())
                    .expect("trait id"),
                value: match &setting.value {
                    pantograph_scheduler::SchedulerTraitValue::String(value) => {
                        DependencyTraitIntentValue::Text(value.clone())
                    }
                    pantograph_scheduler::SchedulerTraitValue::Bool(value) => {
                        DependencyTraitIntentValue::Boolean(*value)
                    }
                    pantograph_scheduler::SchedulerTraitValue::I64(value) => {
                        DependencyTraitIntentValue::Integer(*value)
                    }
                    pantograph_scheduler::SchedulerTraitValue::U64(value) => {
                        DependencyTraitIntentValue::Integer(
                            i64::try_from(*value).expect("trait value fits"),
                        )
                    }
                },
            })
            .collect(),
        caller_context: DependencyPlanningCallerContext {
            source_node_type: Some(
                DependencyNodeTypeId::parse("llm-inference").expect("node type"),
            ),
            workflow_id: Some(task_intent.workflow_id.as_str().to_string()),
            node_id: Some(task_intent.node_id.as_str().to_string()),
            port_id: None,
            run_id: Some(task_intent.workflow_run_id.as_str().to_string()),
        },
    }
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
    let planning_request = dependency_planning_request_for_intent(&task_intent);
    let validated_request = ValidatedDependencyPlanningRequest::try_from(planning_request)
        .expect("validated dependency planning request");
    let requirements_proof = produce_dependency_requirements_proof(&validated_request, None)
        .expect("dependency requirements proof");
    let dependency_readiness_source = crate::workflow::WorkflowSchedulerDependencyReadinessSource {
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
        dependency_requirements_id: requirements_proof.dependency_requirements_id,
        selected_binding_ids: requirements_proof.identity_key.selected_binding_ids,
        dependency_override_fingerprint: requirements_proof.dependency_override_fingerprint,
    };
    WorkflowSchedulerTask {
        workflow_id: task_intent.workflow_id.clone(),
        workflow_run_id: task_intent.workflow_run_id.clone(),
        node_id: task_intent.node_id.clone(),
        task_id: task_intent.task_id.clone(),
        node_type: "llm-inference".to_string(),
        execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
        dependency_task_ids: Vec::new(),
        input_bindings: Vec::new(),
        schedulable_intent_template: Some(WorkflowSchedulerTaskIntentTemplate {
            task_type: task_intent.task_type.clone(),
            constraints: task_intent.constraints.clone(),
            trait_settings: task_intent.trait_settings.clone(),
            dependency_override_patches: task_intent.dependency_override_patches.clone(),
            estimate_hints: task_intent.estimate_hints.clone(),
            dependency_readiness_source,
        }),
        schedulable_intent: Some(task_intent),
        non_runtime_task_template: None,
        source_input_task_template: None,
        inference_descriptor_fingerprint: None,
        runtime_source_context: None,
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
