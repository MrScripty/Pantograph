use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef};
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerNodeId, SchedulerQueueTaskRecord, SchedulerQueueTaskState,
    SchedulerQueueTransitionId, SchedulerRuntimeDeviceConstraints, SchedulerTaskId,
    SchedulerTraitId, SchedulerTraitSetting, SchedulerTraitValue, SchedulerWorkflowId,
    SchedulerWorkflowRunId, SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
    SCHEDULER_QUEUE_STATE_CONTRACT_VERSION,
};

use super::*;

fn scheduler_record(
    workflow_run_id: &str,
    task_id: &str,
    state: SchedulerQueueTaskState,
) -> SchedulerQueueTaskRecord {
    SchedulerQueueTaskRecord {
        contract_version: SCHEDULER_QUEUE_STATE_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow-image-plan").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
        node_id: SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        task_intent: SchedulableTaskIntent {
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
        },
        state,
        state_version: 3,
        last_transition_id: SchedulerQueueTransitionId::parse("transition-ready")
            .expect("transition id"),
    }
}

#[test]
fn scheduler_task_state_read_model_projects_path_free_display_facts() {
    let read_models = workflow_scheduler_task_state_read_models(&[scheduler_record(
        "run-image-plan",
        "image-task",
        SchedulerQueueTaskState::WaitingResources,
    )])
    .expect("task state read model");

    assert_eq!(read_models.len(), 1);
    let read_model = &read_models[0];
    assert_eq!(read_model.workflow_id, "workflow-image-plan");
    assert_eq!(read_model.workflow_run_id, "run-image-plan");
    assert_eq!(read_model.node_id, "image-task");
    assert_eq!(read_model.task_id, "image-task");
    assert_eq!(read_model.task_type, "image_generation");
    assert_eq!(read_model.model_id, "image/example/tiny-diffusion");
    assert_eq!(read_model.state, SchedulerQueueTaskState::WaitingResources);
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
    let read_models = workflow_scheduler_task_state_read_models(&[scheduler_record(
        "run-image-plan",
        "image-task",
        SchedulerQueueTaskState::Ready,
    )])
    .expect("task state read model");
    let serialized = serde_json::to_string(&read_models).expect("serialize read model");

    assert!(!serialized.contains("task_intent"));
    assert!(!serialized.contains("last_transition_id"));
    assert!(!serialized.contains("state_version"));
    assert!(!serialized.contains("model_path"));
    assert!(!serialized.contains("local_load_path"));
    assert!(!serialized.contains("runtime_handoff"));
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
    let workflow_run_id = {
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
            .set_active_run_scheduler_task_records(
                &session_id,
                &workflow_run_id,
                vec![scheduler_record(
                    &workflow_run_id,
                    "image-task",
                    SchedulerQueueTaskState::WaitingResources,
                )],
            )
            .expect("set task records");
        workflow_run_id
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
    assert_eq!(
        response.tasks[0].state,
        SchedulerQueueTaskState::WaitingResources
    );
    assert_eq!(response.tasks[0].task_id, "image-task");
}
