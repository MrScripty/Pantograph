use pantograph_scheduler::SchedulerTaskId;

use crate::scheduler::{
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
