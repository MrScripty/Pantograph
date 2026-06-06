use std::time::Duration;

use super::WorkflowSchedulerQueueWorker;
use crate::scheduler::lifecycle::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
    WorkflowSchedulerLifecycleComponentState, WorkflowSchedulerLifecycleOwnerId,
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

fn scheduler_lifecycle() -> WorkflowSchedulerLifecycleComponentRegistryHandle {
    WorkflowSchedulerLifecycleComponentRegistryHandle::new(
        WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.queue-worker.test")
            .expect("scheduler lifecycle owner id"),
    )
}
