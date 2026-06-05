use pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationState;
use pantograph_scheduler::SchedulerTaskId;

use crate::scheduler::{
    lifecycle::{
        WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
        WorkflowSchedulerLifecycleComponentState, WorkflowSchedulerLifecycleOwnerId,
    },
    task_lifecycle::{
        WorkflowSchedulerTaskLifecycleManager, WorkflowSchedulerTaskLifecycleOwnerId,
        WorkflowSchedulerTaskLifecycleShutdownState,
    },
    WorkflowSchedulerTaskAttemptId,
};

#[test]
fn task_lifecycle_manager_tracks_active_task_handle() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.first");

    let record = manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");

    assert_eq!(
        manager.owner_id().as_str(),
        "workflow-service.lifecycle.test"
    );
    assert_eq!(
        manager.shutdown_state(),
        WorkflowSchedulerTaskLifecycleShutdownState::Running
    );
    assert_eq!(record.owner_id.as_str(), "workflow-service.lifecycle.test");
    assert_eq!(record.task_id, task_id);
    assert_eq!(record.attempt_id, attempt_id);
    assert_eq!(manager.active_task_handle_count(), 1);
    assert!(manager.active_task_handle(&record.task_id).is_some());
    assert_eq!(
        manager
            .runtime_host_dispatch_lifecycle_component()
            .expect("runtime host dispatch component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );
}

#[test]
fn task_lifecycle_manager_rejects_duplicate_active_task_handle() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    manager
        .track_task_handle(task_id.clone(), attempt_id("scheduler-task-attempt.first"))
        .expect("track first task handle");

    let error = manager
        .track_task_handle(task_id, attempt_id("scheduler-task-attempt.second"))
        .expect_err("duplicate task handle must fail");

    assert!(error.to_string().contains("TaskHandleAlreadyTracked"));
    assert_eq!(manager.active_task_handle_count(), 1);
}

#[test]
fn task_lifecycle_manager_rejects_stale_completion() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let active_attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), active_attempt_id.clone())
        .expect("track task handle");

    let error = manager
        .complete_task_handle(&task_id, &attempt_id("scheduler-task-attempt.stale"))
        .expect_err("stale attempt must fail");

    assert!(error.to_string().contains("StaleTaskHandleAttempt"));
    assert_eq!(
        manager
            .active_task_handle(&task_id)
            .expect("active handle")
            .attempt_id,
        active_attempt_id
    );
}

#[test]
fn task_lifecycle_manager_completes_matching_task_handle() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");

    let completed = manager
        .complete_task_handle(&task_id, &attempt_id)
        .expect("complete matching handle");

    assert_eq!(completed.task_id, task_id);
    assert_eq!(completed.attempt_id, attempt_id);
    assert_eq!(manager.active_task_handle_count(), 0);
}

#[test]
fn task_lifecycle_manager_creates_runtime_host_cancellation_signal_for_active_attempt() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");

    let (context, cancellation) = manager
        .runtime_host_cancellation(&task_id, &attempt_id, "runtime-host-request.current")
        .expect("create runtime host cancellation");
    let snapshot = cancellation.snapshot();

    assert_eq!(
        context.cancellation_context_id,
        "runtime-host-cancellation.runtime-host-request.current"
    );
    assert_eq!(
        snapshot.cancellation_context_id,
        context.cancellation_context_id
    );
    assert_eq!(
        snapshot.state,
        RuntimeHostExecutionCancellationState::Running
    );
    assert_eq!(snapshot.reason, None);
}

#[test]
fn task_lifecycle_manager_rejects_runtime_host_cancellation_for_stale_attempt() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    manager
        .track_task_handle(
            task_id.clone(),
            attempt_id("scheduler-task-attempt.current"),
        )
        .expect("track task handle");

    let error = match manager.runtime_host_cancellation(
        &task_id,
        &attempt_id("scheduler-task-attempt.stale"),
        "runtime-host-request.stale",
    ) {
        Ok(_) => panic!("stale attempt must not receive cancellation handle"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("StaleTaskHandleAttempt"));
}

#[test]
fn task_lifecycle_manager_updates_runtime_host_cancellation_signal() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");
    let (_context, cancellation) = manager
        .runtime_host_cancellation(&task_id, &attempt_id, "runtime-host-request.current")
        .expect("create runtime host cancellation");

    manager
        .request_task_cancellation(&task_id, &attempt_id, "user cancelled task")
        .expect("request task cancellation");

    let snapshot = cancellation.snapshot();
    assert_eq!(
        snapshot.state,
        RuntimeHostExecutionCancellationState::CancellationRequested
    );
    assert_eq!(snapshot.reason.as_deref(), Some("user cancelled task"));
}

#[test]
fn task_lifecycle_manager_applies_pending_cancellation_to_later_runtime_host_signal() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");

    manager
        .request_task_cancellation(&task_id, &attempt_id, "user cancelled before dispatch")
        .expect("request task cancellation");
    let (_context, cancellation) = manager
        .runtime_host_cancellation(&task_id, &attempt_id, "runtime-host-request.current")
        .expect("create runtime host cancellation");

    let snapshot = cancellation.snapshot();
    assert_eq!(
        snapshot.state,
        RuntimeHostExecutionCancellationState::CancellationRequested
    );
    assert_eq!(
        snapshot.reason.as_deref(),
        Some("user cancelled before dispatch")
    );
    assert_eq!(manager.active_task_handle_count(), 1);
}

#[test]
fn task_lifecycle_manager_shutdown_updates_runtime_host_cancellation_signal() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");
    let (_context, cancellation) = manager
        .runtime_host_cancellation(&task_id, &attempt_id, "runtime-host-request.current")
        .expect("create runtime host cancellation");

    manager.begin_shutdown();

    let snapshot = cancellation.snapshot();
    assert_eq!(
        snapshot.state,
        RuntimeHostExecutionCancellationState::ShutdownRequested
    );
    assert_eq!(
        snapshot.reason.as_deref(),
        Some("workflow-service task lifecycle owner is shutting down")
    );
}

#[test]
fn task_lifecycle_manager_applies_pending_shutdown_to_later_runtime_host_signal() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");

    manager.begin_shutdown();
    let (_context, cancellation) = manager
        .runtime_host_cancellation(&task_id, &attempt_id, "runtime-host-request.current")
        .expect("create runtime host cancellation");

    let snapshot = cancellation.snapshot();
    assert_eq!(
        snapshot.state,
        RuntimeHostExecutionCancellationState::ShutdownRequested
    );
    assert_eq!(
        snapshot.reason.as_deref(),
        Some("workflow-service task lifecycle owner is shutting down")
    );
    assert_eq!(manager.active_task_handle_count(), 1);
}

#[test]
fn task_lifecycle_manager_shutdown_is_idempotent_and_blocks_new_handles() {
    let mut manager = lifecycle_manager();

    assert_eq!(
        manager.begin_shutdown(),
        WorkflowSchedulerTaskLifecycleShutdownState::ShuttingDown
    );
    assert_eq!(
        manager.begin_shutdown(),
        WorkflowSchedulerTaskLifecycleShutdownState::ShuttingDown
    );
    let error = manager
        .track_task_handle(
            task_id("image-task"),
            attempt_id("scheduler-task-attempt.after-shutdown"),
        )
        .expect_err("shutdown owner must reject new handles");

    assert!(error.to_string().contains("LifecycleOwnerShuttingDown"));
    assert_eq!(
        manager.finish_shutdown().expect("finish shutdown"),
        WorkflowSchedulerTaskLifecycleShutdownState::Shutdown
    );
    assert_eq!(
        manager.begin_shutdown(),
        WorkflowSchedulerTaskLifecycleShutdownState::Shutdown
    );
}

#[test]
fn task_lifecycle_manager_refuses_final_shutdown_with_active_handles() {
    let mut manager = lifecycle_manager();
    manager
        .track_task_handle(
            task_id("image-task"),
            attempt_id("scheduler-task-attempt.current"),
        )
        .expect("track task handle");
    manager.begin_shutdown();

    let error = manager
        .finish_shutdown()
        .expect_err("active handles must block final shutdown");

    assert!(error.to_string().contains("ActiveTaskHandlesRemain"));
}

#[tokio::test]
async fn task_lifecycle_manager_aborts_tracked_task_supervisors() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");
    let join_handle = tokio::spawn(async {
        std::future::pending::<()>().await;
    });

    manager
        .track_task_supervisor_abort_handle(&task_id, &attempt_id, join_handle.abort_handle())
        .expect("track supervisor abort handle");
    manager.begin_shutdown();

    assert_eq!(manager.abort_task_supervisors(), 1);
    let error = join_handle
        .await
        .expect_err("supervisor join should be cancelled");
    assert!(error.is_cancelled());
}

#[tokio::test]
async fn task_lifecycle_manager_marks_runtime_host_dispatch_running_for_supervisor_handle() {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");
    let join_handle = tokio::spawn(async {
        std::future::pending::<()>().await;
    });

    manager
        .track_task_supervisor_abort_handle(&task_id, &attempt_id, join_handle.abort_handle())
        .expect("track supervisor abort handle");

    let component = manager
        .runtime_host_dispatch_lifecycle_component()
        .expect("runtime host dispatch component");
    assert_eq!(
        component.component,
        WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch
    );
    assert_eq!(
        component.state,
        WorkflowSchedulerLifecycleComponentState::Running
    );

    join_handle.abort();
    let _ = join_handle.await;
}

#[tokio::test]
async fn task_lifecycle_manager_marks_runtime_host_dispatch_not_started_after_supervisor_completion(
) {
    let mut manager = lifecycle_manager();
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");
    let join_handle = tokio::spawn(async {
        std::future::pending::<()>().await;
    });
    manager
        .track_task_supervisor_abort_handle(&task_id, &attempt_id, join_handle.abort_handle())
        .expect("track supervisor abort handle");

    manager
        .complete_task_handle(&task_id, &attempt_id)
        .expect("complete task handle");

    assert_eq!(
        manager
            .runtime_host_dispatch_lifecycle_component()
            .expect("runtime host dispatch component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );

    join_handle.abort();
    let _ = join_handle.await;
}

#[test]
fn task_lifecycle_manager_marks_runtime_host_dispatch_shutdown_states() {
    let mut manager = lifecycle_manager();

    manager.begin_shutdown();

    assert_eq!(
        manager
            .runtime_host_dispatch_lifecycle_component()
            .expect("runtime host dispatch component")
            .state,
        WorkflowSchedulerLifecycleComponentState::ShuttingDown
    );
    manager.finish_shutdown().expect("finish shutdown");
    assert_eq!(
        manager
            .runtime_host_dispatch_lifecycle_component()
            .expect("runtime host dispatch component")
            .state,
        WorkflowSchedulerLifecycleComponentState::Shutdown
    );
}

#[tokio::test]
async fn task_lifecycle_manager_updates_shared_scheduler_lifecycle_registry() {
    let scheduler_lifecycle = WorkflowSchedulerLifecycleComponentRegistryHandle::new(
        WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.shared-lifecycle.test")
            .expect("scheduler lifecycle owner id"),
    );
    let mut manager = WorkflowSchedulerTaskLifecycleManager::new_with_scheduler_lifecycle(
        WorkflowSchedulerTaskLifecycleOwnerId::parse("workflow-service.lifecycle.test")
            .expect("task lifecycle owner id"),
        scheduler_lifecycle.clone(),
    );
    let task_id = task_id("image-task");
    let attempt_id = attempt_id("scheduler-task-attempt.current");
    let join_handle = tokio::spawn(async {
        std::future::pending::<()>().await;
    });

    manager
        .track_task_handle(task_id.clone(), attempt_id.clone())
        .expect("track task handle");
    manager
        .track_task_supervisor_abort_handle(&task_id, &attempt_id, join_handle.abort_handle())
        .expect("track supervisor abort handle");

    assert_eq!(
        scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch)
            .expect("runtime-host dispatch shared component")
            .state,
        WorkflowSchedulerLifecycleComponentState::Running
    );
    assert_eq!(
        scheduler_lifecycle
            .required_component_records()
            .expect("required component records")
            .len(),
        WorkflowSchedulerLifecycleComponentKind::required_components().len()
    );

    join_handle.abort();
    let _ = join_handle.await;
}

#[test]
fn task_lifecycle_owner_id_rejects_blank_value() {
    let error =
        WorkflowSchedulerTaskLifecycleOwnerId::parse(" ").expect_err("blank owner id must fail");

    assert!(error.to_string().contains("InvalidLifecycleOwnerId"));
}

fn lifecycle_manager() -> WorkflowSchedulerTaskLifecycleManager {
    WorkflowSchedulerTaskLifecycleManager::new(
        WorkflowSchedulerTaskLifecycleOwnerId::parse("workflow-service.lifecycle.test")
            .expect("owner id"),
    )
}

fn task_id(value: &str) -> SchedulerTaskId {
    SchedulerTaskId::parse(value).expect("task id")
}

fn attempt_id(value: &str) -> WorkflowSchedulerTaskAttemptId {
    WorkflowSchedulerTaskAttemptId::parse(value).expect("attempt id")
}
