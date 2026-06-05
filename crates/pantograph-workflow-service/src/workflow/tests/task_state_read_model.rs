use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef};
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerNodeId, SchedulerNonRuntimeTaskIntent,
    SchedulerNonRuntimeTaskKind, SchedulerRuntimeDeviceConstraints, SchedulerTaskExecutionIntent,
    SchedulerTaskId, SchedulerTaskState, SchedulerTaskStateDiagnostic,
    SchedulerTaskStateDiagnosticCode, SchedulerTaskStateDiagnosticSeverity, SchedulerTaskStateKind,
    SchedulerTaskStateRecord, SchedulerTaskStateTransitionId, SchedulerTraitId,
    SchedulerTraitSetting, SchedulerTraitValue, SchedulerWorkflowId, SchedulerWorkflowRunId,
    SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};

use super::*;

fn scheduler_record(
    workflow_run_id: &str,
    task_id: &str,
    state: SchedulerTaskStateKind,
) -> SchedulerTaskStateRecord {
    let intent = SchedulableTaskIntent {
        contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
        node_id: SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        fairness_key: None,
        task_type: DependencyTaskId::parse("image_generation").expect("task type"),
        model_ref: PumasModelRef {
            model_id: "image/example/tiny-diffusion".to_string(),
            revision: None,
            selected_artifact_id: Some("diffusers-bundle".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        },
        constraints: SchedulerRuntimeDeviceConstraints {
            requested_runtime_id: Some("diffusers-pytorch".parse().expect("runtime id")),
            requested_device_id: Some("cuda:0".parse().expect("device id")),
        },
        trait_settings: vec![SchedulerTraitSetting {
            trait_id: SchedulerTraitId::parse("denoising_scheduler").expect("trait id"),
            value: SchedulerTraitValue::String("euler".to_string()),
        }],
        dependency_override_patches: Vec::new(),
        estimate_hints: Vec::new(),
    };
    SchedulerTaskStateRecord {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
        node_id: SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        state: state_with_intent(state, intent),
        state_version: 3,
        last_transition_id: SchedulerTaskStateTransitionId::parse("transition-ready")
            .expect("transition id"),
    }
}

fn state_with_intent(
    state: SchedulerTaskStateKind,
    task_intent: SchedulableTaskIntent,
) -> SchedulerTaskState {
    match state {
        SchedulerTaskStateKind::Ready => SchedulerTaskState::Ready {
            execution_intent: runtime_execution_intent(task_intent),
        },
        SchedulerTaskStateKind::WaitingDependencyReadiness => {
            SchedulerTaskState::WaitingDependencyReadiness {
                execution_intent: runtime_execution_intent(task_intent),
            }
        }
        SchedulerTaskStateKind::WaitingResources => SchedulerTaskState::WaitingResources {
            execution_intent: runtime_execution_intent(task_intent),
        },
        SchedulerTaskStateKind::WaitingBatch => SchedulerTaskState::WaitingBatch {
            execution_intent: runtime_execution_intent(task_intent),
        },
        SchedulerTaskStateKind::Running => SchedulerTaskState::Running {
            execution_intent: runtime_execution_intent(task_intent),
        },
        SchedulerTaskStateKind::Completed => SchedulerTaskState::Completed {
            execution_intent: runtime_execution_intent(task_intent),
        },
        other => panic!("test helper expected schedulable state, got {other:?}"),
    }
}

fn runtime_execution_intent(task_intent: SchedulableTaskIntent) -> SchedulerTaskExecutionIntent {
    SchedulerTaskExecutionIntent::Runtime { task_intent }
}

fn scheduler_task_graph(workflow_run_id: &str) -> WorkflowSchedulerTaskGraph {
    let workflow_id = SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id");
    let workflow_run_id = SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id");
    let image_task_id = SchedulerTaskId::parse("image-task").expect("task id");
    let upstream_task_id = SchedulerTaskId::parse("prompt-task").expect("task id");
    WorkflowSchedulerTaskGraph {
        schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        workflow_id: workflow_id.clone(),
        workflow_run_id: workflow_run_id.clone(),
        tasks: vec![WorkflowSchedulerTask {
            workflow_id,
            workflow_run_id,
            node_id: SchedulerNodeId::parse("image-task").expect("node id"),
            task_id: image_task_id,
            node_type: "llm-inference".to_string(),
            execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            dependency_task_ids: vec![upstream_task_id.clone()],
            input_bindings: vec![WorkflowSchedulerTaskInputBinding {
                source_node_id: SchedulerNodeId::parse("prompt-task").expect("node id"),
                source_task_id: upstream_task_id,
                source_port_id: "prompt".to_string(),
                target_port_id: "prompt".to_string(),
            }],
            schedulable_intent: None,
            schedulable_intent_template: None,
            non_runtime_task_template: None,
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            diagnostics: Vec::new(),
        }],
    }
}

#[test]
fn scheduler_task_state_read_model_projects_path_free_display_facts() {
    let task_graph = scheduler_task_graph("run-image-plan");
    let read_models = workflow_scheduler_task_state_read_models(
        &task_graph,
        &[scheduler_record(
            "run-image-plan",
            "image-task",
            SchedulerTaskStateKind::WaitingResources,
        )],
    )
    .expect("task state read model");

    assert_eq!(read_models.len(), 1);
    let read_model = &read_models[0];
    assert_eq!(read_model.workflow_id, "workflow-image-plan");
    assert_eq!(read_model.workflow_run_id, "run-image-plan");
    assert_eq!(read_model.node_id, "image-task");
    assert_eq!(read_model.task_id, "image-task");
    assert_eq!(read_model.node_type, "llm-inference");
    assert_eq!(read_model.dependency_task_ids, vec!["prompt-task"]);
    assert_eq!(read_model.input_bindings.len(), 1);
    assert_eq!(read_model.input_bindings[0].source_task_id, "prompt-task");
    assert_eq!(read_model.input_bindings[0].target_port_id, "prompt");
    assert_eq!(
        read_model.execution_kind,
        Some(WorkflowSchedulerTaskStateExecutionKind::Runtime)
    );
    assert_eq!(read_model.task_type.as_deref(), Some("image_generation"));
    assert_eq!(
        read_model.model_id.as_deref(),
        Some("image/example/tiny-diffusion")
    );
    assert_eq!(read_model.non_runtime_task_kind, None);
    assert_eq!(read_model.state, SchedulerTaskStateKind::WaitingResources);
    assert_eq!(
        read_model.requested_runtime_id.as_deref(),
        Some("diffusers-pytorch")
    );
    assert_eq!(read_model.requested_device_id.as_deref(), Some("cuda:0"));
    assert_eq!(read_model.trait_settings.len(), 1);
    assert_eq!(read_model.trait_settings[0].trait_id, "denoising_scheduler");
}

#[test]
fn scheduler_task_state_read_model_does_not_expose_execution_internals() {
    let task_graph = scheduler_task_graph("run-image-plan");
    let read_models = workflow_scheduler_task_state_read_models(
        &task_graph,
        &[scheduler_record(
            "run-image-plan",
            "image-task",
            SchedulerTaskStateKind::Ready,
        )],
    )
    .expect("task state read model");
    let serialized = serde_json::to_string(&read_models).expect("serialize read model");

    assert!(!serialized.contains("task_intent"));
    assert!(!serialized.contains("last_transition_id"));
    assert!(!serialized.contains("state_version"));
    assert!(!serialized.contains("model_path"));
    assert!(!serialized.contains("local_load_path"));
    assert!(!serialized.contains("runtime_handoff"));
}

#[test]
fn scheduler_task_state_read_model_supports_pre_intent_state() {
    let record = SchedulerTaskStateRecord {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse("run-image-plan").expect("run id"),
        node_id: SchedulerNodeId::parse("image-task").expect("node id"),
        task_id: SchedulerTaskId::parse("image-task").expect("task id"),
        state: SchedulerTaskState::AwaitingInputs {
            diagnostics: Vec::new(),
        },
        state_version: 1,
        last_transition_id: SchedulerTaskStateTransitionId::parse("transition-awaiting-inputs")
            .expect("transition id"),
    };

    let task_graph = scheduler_task_graph("run-image-plan");
    let read_models = workflow_scheduler_task_state_read_models(&task_graph, &[record])
        .expect("task state read model");

    assert_eq!(read_models.len(), 1);
    let read_model = &read_models[0];
    assert_eq!(read_model.state, SchedulerTaskStateKind::AwaitingInputs);
    assert_eq!(read_model.execution_kind, None);
    assert_eq!(read_model.task_type, None);
    assert_eq!(read_model.model_id, None);
    assert_eq!(read_model.non_runtime_task_kind, None);
    assert_eq!(read_model.requested_runtime_id, None);
    assert_eq!(read_model.requested_device_id, None);
    assert!(read_model.trait_settings.is_empty());
}

#[test]
fn scheduler_task_state_read_model_projects_state_diagnostics() {
    let record = SchedulerTaskStateRecord {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse("run-image-plan").expect("run id"),
        node_id: SchedulerNodeId::parse("image-task").expect("node id"),
        task_id: SchedulerTaskId::parse("image-task").expect("task id"),
        state: SchedulerTaskState::InputUnavailable {
            diagnostics: vec![SchedulerTaskStateDiagnostic {
                severity: SchedulerTaskStateDiagnosticSeverity::Error,
                code: SchedulerTaskStateDiagnosticCode::InputUnavailable,
                message: "required prompt output is unavailable".to_string(),
                hint: Some("retry after upstream task succeeds".to_string()),
            }],
        },
        state_version: 2,
        last_transition_id: SchedulerTaskStateTransitionId::parse("transition-input-unavailable")
            .expect("transition id"),
    };

    let task_graph = scheduler_task_graph("run-image-plan");
    let read_models = workflow_scheduler_task_state_read_models(&task_graph, &[record])
        .expect("task state read model");

    let read_model = &read_models[0];
    assert_eq!(read_model.state, SchedulerTaskStateKind::InputUnavailable);
    assert_eq!(read_model.state_diagnostics.len(), 1);
    assert_eq!(
        read_model.state_diagnostics[0].code,
        SchedulerTaskStateDiagnosticCode::InputUnavailable
    );
    assert_eq!(
        read_model.state_diagnostics[0].hint.as_deref(),
        Some("retry after upstream task succeeds")
    );
    assert_eq!(read_model.execution_kind, None);
}

#[test]
fn scheduler_task_state_read_model_projects_non_runtime_execution_kind() {
    let record = SchedulerTaskStateRecord {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse("run-image-plan").expect("run id"),
        node_id: SchedulerNodeId::parse("image-task").expect("node id"),
        task_id: SchedulerTaskId::parse("image-task").expect("task id"),
        state: SchedulerTaskState::Ready {
            execution_intent: SchedulerTaskExecutionIntent::NonRuntime {
                task_intent: SchedulerNonRuntimeTaskIntent {
                    contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
                    workflow_id: SchedulerWorkflowId::parse("workflow-image-plan")
                        .expect("workflow id"),
                    workflow_run_id: SchedulerWorkflowRunId::parse("run-image-plan")
                        .expect("run id"),
                    node_id: SchedulerNodeId::parse("image-task").expect("node id"),
                    task_id: SchedulerTaskId::parse("image-task").expect("task id"),
                    task_kind: SchedulerNonRuntimeTaskKind::parse("text_output")
                        .expect("task kind"),
                },
            },
        },
        state_version: 2,
        last_transition_id: SchedulerTaskStateTransitionId::parse("transition-non-runtime-ready")
            .expect("transition id"),
    };

    let task_graph = scheduler_task_graph("run-image-plan");
    let read_models = workflow_scheduler_task_state_read_models(&task_graph, &[record])
        .expect("task state read model");

    let read_model = &read_models[0];
    assert_eq!(
        read_model.execution_kind,
        Some(WorkflowSchedulerTaskStateExecutionKind::NonRuntime)
    );
    assert_eq!(read_model.task_type, None);
    assert_eq!(read_model.model_id, None);
    assert_eq!(
        read_model.non_runtime_task_kind.as_deref(),
        Some("text_output")
    );
}

#[test]
fn scheduler_task_state_read_model_projects_source_input_execution_kind() {
    let mut task_graph = scheduler_task_graph("run-image-plan");
    let task = task_graph.tasks.first_mut().expect("task");
    task.node_type = "text-input".to_string();
    task.execution_class = WorkflowSchedulerTaskExecutionClass::SourceInput;
    task.schedulable_intent = None;
    task.schedulable_intent_template = None;

    let read_models = workflow_scheduler_task_state_read_models(
        &task_graph,
        &[SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse("run-image-plan").expect("run id"),
            node_id: SchedulerNodeId::parse("image-task").expect("node id"),
            task_id: SchedulerTaskId::parse("image-task").expect("task id"),
            state: SchedulerTaskState::AwaitingInputs {
                diagnostics: Vec::new(),
            },
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse("transition-source-input")
                .expect("transition id"),
        }],
    )
    .expect("task state read model");

    assert_eq!(
        read_models[0].execution_kind,
        Some(WorkflowSchedulerTaskStateExecutionKind::SourceInput)
    );
    assert_eq!(read_models[0].task_type, None);
    assert_eq!(read_models[0].model_id, None);
    assert_eq!(read_models[0].non_runtime_task_kind, None);
}

#[test]
fn scheduler_task_state_read_model_fails_when_task_graph_and_state_diverge() {
    let task_graph = scheduler_task_graph("run-image-plan");
    let error = workflow_scheduler_task_state_read_models(&task_graph, &[])
        .expect_err("missing state record should fail closed");
    assert!(error.to_string().contains("missing task-state records"));

    let empty_task_graph = WorkflowSchedulerTaskGraph {
        tasks: Vec::new(),
        ..task_graph
    };
    let error = workflow_scheduler_task_state_read_models(
        &empty_task_graph,
        &[scheduler_record(
            "run-image-plan",
            "different-task",
            SchedulerTaskStateKind::Ready,
        )],
    )
    .expect_err("extra state record should fail closed");
    assert!(error.to_string().contains("records outside the task graph"));
}

#[tokio::test]
async fn scheduler_task_state_query_reads_active_run_records() {
    let service = WorkflowService::new();
    let session_id = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        store
            .create_session(
                "workflow-image-plan".to_string(),
                None,
                None,
                vec!["pytorch".to_string()],
                vec!["image/example/tiny-diffusion".to_string()],
                true,
            )
            .expect("create session")
    };
    let (workflow_run_id, active_attempt_id) = {
        let mut store = service
            .session_store
            .lock()
            .expect("session store lock poisoned");
        let request = WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
            timeout_ms: None,
            priority: None,
        };
        let workflow_run_id = store
            .enqueue_run(&session_id, &request)
            .expect("enqueue run");
        store
            .begin_queued_run(&session_id, &workflow_run_id)
            .expect("begin queued run")
            .expect("dequeued run");
        store
            .set_active_run_scheduler_task_state(
                &session_id,
                &workflow_run_id,
                scheduler_task_graph(&workflow_run_id),
                vec![scheduler_record(
                    &workflow_run_id,
                    "image-task",
                    SchedulerTaskStateKind::Ready,
                )],
            )
            .expect("set task records");
        let started = service
            .scheduler_task_orchestrator
            .start_ready_runtime_task(&mut store, &session_id, &workflow_run_id, "image-task")
            .expect("start runtime scheduler task attempt");
        (workflow_run_id, started.attempt_id().as_str().to_string())
    };

    let response = service
        .workflow_get_scheduler_task_state_read_models(
            WorkflowSchedulerTaskStateReadModelQueryRequest {
                session_id: session_id.clone(),
                workflow_run_id: workflow_run_id.clone(),
            },
        )
        .await
        .expect("task state query");

    assert_eq!(response.session_id, session_id);
    assert_eq!(response.workflow_run_id, workflow_run_id);
    assert_eq!(response.tasks.len(), 1);
    assert_eq!(response.tasks[0].state, SchedulerTaskStateKind::Running);
    assert_eq!(response.tasks[0].task_id, "image-task");
    assert_eq!(
        response.tasks[0].active_attempt_id.as_deref(),
        Some(active_attempt_id.as_str())
    );
    assert!(
        response.tasks[0].active_attempt_started_at_ms.is_some(),
        "running task should expose active attempt start time"
    );
}
