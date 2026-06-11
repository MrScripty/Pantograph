use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::WorkflowSchedulerQueueAdmissionCommand;
use super::WorkflowSchedulerQueueTaskStateCommand;
use super::WorkflowSchedulerQueueWorker;
use crate::scheduler::lifecycle::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
    WorkflowSchedulerLifecycleComponentState, WorkflowSchedulerLifecycleOwnerId,
};
use crate::scheduler::WorkflowExecutionSessionStore;
use crate::workflow::{
    WorkflowExecutionSessionRunRequest, WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskGraph,
};
use pantograph_scheduler::{
    SchedulerNodeId, SchedulerTaskId, SchedulerTaskState, SchedulerTaskStateKind,
    SchedulerTaskStateRecord, SchedulerTaskStateTransitionId, SchedulerWorkflowId,
    SchedulerWorkflowRunId, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};

#[tokio::test]
async fn queue_worker_marks_running_until_shutdown() {
    let scheduler_lifecycle = scheduler_lifecycle();
    let worker = WorkflowSchedulerQueueWorker::spawn(scheduler_lifecycle.clone())
        .expect("spawn scheduler queue worker");

    assert_eq!(
        scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::QueueWorker)
            .expect("queue worker component")
            .state,
        WorkflowSchedulerLifecycleComponentState::Running
    );

    worker
        .shutdown()
        .await
        .expect("shutdown scheduler queue worker");

    assert_eq!(
        scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::QueueWorker)
            .expect("queue worker component")
            .state,
        WorkflowSchedulerLifecycleComponentState::Shutdown
    );
}

#[tokio::test]
async fn queue_worker_wake_is_observed_without_public_lifecycle_projection() {
    let scheduler_lifecycle = scheduler_lifecycle();
    let worker = WorkflowSchedulerQueueWorker::spawn(scheduler_lifecycle.clone())
        .expect("spawn scheduler queue worker");

    worker.wake();

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if worker.observed_wake_count() > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("queue worker should observe wake");

    assert_eq!(
        scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::QueueWorker)
            .expect("queue worker component")
            .state,
        WorkflowSchedulerLifecycleComponentState::Running
    );

    worker
        .shutdown()
        .await
        .expect("shutdown scheduler queue worker");
}

#[tokio::test]
async fn queue_worker_shutdown_is_idempotent() {
    let scheduler_lifecycle = scheduler_lifecycle();
    let worker = WorkflowSchedulerQueueWorker::spawn(scheduler_lifecycle.clone())
        .expect("spawn scheduler queue worker");

    worker
        .shutdown()
        .await
        .expect("first shutdown should complete");
    worker
        .shutdown()
        .await
        .expect("second shutdown should complete");

    assert_eq!(
        scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::QueueWorker)
            .expect("queue worker component")
            .state,
        WorkflowSchedulerLifecycleComponentState::Shutdown
    );
}

#[test]
fn queue_worker_spawn_requires_active_tokio_runtime() {
    let error = WorkflowSchedulerQueueWorker::spawn(scheduler_lifecycle())
        .expect_err("queue worker spawn should require runtime");

    assert!(
        error
            .to_string()
            .contains("scheduler queue worker requires an active Tokio runtime"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn queue_worker_admits_queued_run() {
    let session_store = Arc::new(Mutex::new(WorkflowExecutionSessionStore::new(2, 2)));
    let session_id = {
        let mut store = session_store.lock().expect("session store lock");
        let session_id = store
            .create_session(
                "wf-queue-worker".to_string(),
                None,
                None,
                Vec::new(),
                Vec::new(),
                false,
            )
            .expect("create session");
        store
            .enqueue_run(&session_id, &empty_run_request(&session_id))
            .expect("enqueue run");
        session_id
    };
    let workflow_run_id = {
        let store = session_store.lock().expect("session store lock");
        store
            .list_queue(&session_id)
            .expect("list queue")
            .first()
            .expect("queued run")
            .workflow_run_id
            .clone()
    };

    let queued_run = WorkflowSchedulerQueueWorker::admit_queued_run(
        WorkflowSchedulerQueueAdmissionCommand::new(
            session_store,
            session_id.clone(),
            workflow_run_id.clone(),
        ),
    )
    .await
    .expect("admit queued run");

    assert_eq!(queued_run.queued.workflow_run_id, workflow_run_id);
    assert_eq!(queued_run.workflow_id, "wf-queue-worker");
}

#[tokio::test]
async fn queue_worker_admission_waits_until_active_run_finishes() {
    let session_store = Arc::new(Mutex::new(WorkflowExecutionSessionStore::new(2, 2)));
    let (session_id, first_run_id, second_run_id) = {
        let mut store = session_store.lock().expect("session store lock");
        let session_id = store
            .create_session(
                "wf-queue-worker".to_string(),
                None,
                None,
                Vec::new(),
                Vec::new(),
                false,
            )
            .expect("create session");
        let first_run_id = store
            .enqueue_run(&session_id, &empty_run_request(&session_id))
            .expect("enqueue first run");
        let second_run_id = store
            .enqueue_run(&session_id, &empty_run_request(&session_id))
            .expect("enqueue second run");
        store
            .begin_queued_run(&session_id, &first_run_id)
            .expect("begin first run")
            .expect("first run admitted");
        (session_id, first_run_id, second_run_id)
    };

    let admission = tokio::spawn(WorkflowSchedulerQueueWorker::admit_queued_run(
        WorkflowSchedulerQueueAdmissionCommand::new(
            session_store.clone(),
            session_id.clone(),
            second_run_id.clone(),
        ),
    ));
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert!(
        !admission.is_finished(),
        "second run should wait while the first run is active"
    );

    {
        let mut store = session_store.lock().expect("session store lock");
        store
            .finish_run(&session_id, &first_run_id)
            .expect("finish first run");
    }
    let queued_run = tokio::time::timeout(Duration::from_secs(1), admission)
        .await
        .expect("admission should finish after active run")
        .expect("admission task should not panic")
        .expect("admit second run");

    assert_eq!(queued_run.queued.workflow_run_id, second_run_id);
}

#[tokio::test]
async fn queue_worker_initializes_admitted_task_state() {
    let session_store = Arc::new(Mutex::new(WorkflowExecutionSessionStore::new(2, 2)));
    let (session_id, workflow_run_id) = {
        let mut store = session_store.lock().expect("session store lock");
        let session_id = store
            .create_session(
                "wf-queue-worker".to_string(),
                None,
                None,
                Vec::new(),
                Vec::new(),
                false,
            )
            .expect("create session");
        let workflow_run_id = store
            .enqueue_run(&session_id, &empty_run_request(&session_id))
            .expect("enqueue run");
        store
            .begin_queued_run(&session_id, &workflow_run_id)
            .expect("begin queued run")
            .expect("admitted run");
        (session_id, workflow_run_id)
    };
    let task_graph = scheduler_task_graph(&workflow_run_id);
    let records = vec![scheduler_record(&workflow_run_id, "task-text-input")];

    WorkflowSchedulerQueueWorker::initialize_admitted_task_state(
        WorkflowSchedulerQueueTaskStateCommand::new(
            session_store.clone(),
            session_id.clone(),
            workflow_run_id.clone(),
            task_graph,
            records,
        ),
    )
    .expect("initialize admitted task state");

    let (_, stored_records) = {
        let store = session_store.lock().expect("session store lock");
        store
            .active_run_scheduler_task_state(&session_id, &workflow_run_id)
            .expect("active task state")
            .expect("task state should be stored")
    };
    assert_eq!(stored_records.len(), 1);
    assert_eq!(stored_records[0].task_id.as_str(), "task-text-input");
    assert_eq!(
        stored_records[0].state.kind(),
        SchedulerTaskStateKind::AwaitingInputs
    );
}

fn scheduler_lifecycle() -> WorkflowSchedulerLifecycleComponentRegistryHandle {
    WorkflowSchedulerLifecycleComponentRegistryHandle::new(
        WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.queue-worker.test")
            .expect("scheduler lifecycle owner id"),
    )
}

fn scheduler_task_graph(workflow_run_id: &str) -> WorkflowSchedulerTaskGraph {
    let workflow_id = SchedulerWorkflowId::parse("wf-queue-worker").expect("workflow id");
    let workflow_run_id = SchedulerWorkflowRunId::parse(workflow_run_id).expect("workflow run id");
    let node_id = SchedulerNodeId::parse("task-text-input").expect("node id");
    let task_id = SchedulerTaskId::parse("task-text-input").expect("task id");
    WorkflowSchedulerTaskGraph {
        schema_version: crate::workflow::WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        workflow_id: workflow_id.clone(),
        workflow_run_id: workflow_run_id.clone(),
        tasks: vec![WorkflowSchedulerTask {
            workflow_id,
            workflow_run_id,
            node_id,
            task_id,
            node_type: "source.input.text".to_string(),
            execution_class: WorkflowSchedulerTaskExecutionClass::SourceInput,
            dependency_task_ids: Vec::new(),
            input_bindings: Vec::new(),
            schedulable_intent: None,
            schedulable_intent_template: None,
            non_runtime_task_template: None,
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            runtime_source_context: None,
            diagnostics: Vec::new(),
        }],
    }
}

fn scheduler_record(workflow_run_id: &str, task_id: &str) -> SchedulerTaskStateRecord {
    SchedulerTaskStateRecord {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("wf-queue-worker").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("workflow run id"),
        node_id: SchedulerNodeId::parse(task_id).expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        state: SchedulerTaskState::AwaitingInputs {
            diagnostics: Vec::new(),
        },
        state_version: 1,
        last_transition_id: SchedulerTaskStateTransitionId::parse("initial:task-text-input")
            .expect("transition id"),
    }
}

fn empty_run_request(session_id: &str) -> WorkflowExecutionSessionRunRequest {
    WorkflowExecutionSessionRunRequest {
        session_id: session_id.to_string(),
        workflow_semantic_version: "0.1.0".to_string(),
        inputs: Vec::new(),
        output_targets: None,
        override_selection: None,
        timeout_ms: None,
        priority: None,
    }
}
