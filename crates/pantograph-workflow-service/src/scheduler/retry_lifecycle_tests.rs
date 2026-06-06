use super::WorkflowSchedulerRetryLifecycle;
use crate::scheduler::lifecycle::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
    WorkflowSchedulerLifecycleComponentState, WorkflowSchedulerLifecycleOwnerId,
};
use crate::workflow::WorkflowServiceError;

#[test]
fn retry_lifecycle_marks_retry_loop_running_during_retry_action() {
    let scheduler_lifecycle = scheduler_lifecycle();
    let retry_lifecycle = WorkflowSchedulerRetryLifecycle::new(scheduler_lifecycle.clone());

    retry_lifecycle
        .run_retry_loop(|| {
            assert_eq!(
                scheduler_lifecycle
                    .component(WorkflowSchedulerLifecycleComponentKind::RetryLoop)
                    .expect("retry loop component")
                    .state,
                WorkflowSchedulerLifecycleComponentState::Running
            );
            Ok(())
        })
        .expect("retry loop action should complete");

    assert_eq!(
        retry_lifecycle
            .retry_loop_lifecycle_component()
            .expect("retry loop lifecycle component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );
}

#[test]
fn retry_lifecycle_resets_retry_loop_after_retry_action_error() {
    let scheduler_lifecycle = scheduler_lifecycle();
    let retry_lifecycle = WorkflowSchedulerRetryLifecycle::new(scheduler_lifecycle.clone());

    let error = retry_lifecycle
        .run_retry_loop(|| {
            assert_eq!(
                scheduler_lifecycle
                    .component(WorkflowSchedulerLifecycleComponentKind::RetryLoop)
                    .expect("retry loop component")
                    .state,
                WorkflowSchedulerLifecycleComponentState::Running
            );
            Err::<(), WorkflowServiceError>(WorkflowServiceError::Internal(
                "retry action failed".to_string(),
            ))
        })
        .expect_err("retry loop action should return primary error");

    assert!(
        error.to_string().contains("retry action failed"),
        "unexpected error: {error}"
    );
    assert_eq!(
        retry_lifecycle
            .retry_loop_lifecycle_component()
            .expect("retry loop lifecycle component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );
}

fn scheduler_lifecycle() -> WorkflowSchedulerLifecycleComponentRegistryHandle {
    WorkflowSchedulerLifecycleComponentRegistryHandle::new(
        WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.retry-lifecycle.test")
            .expect("scheduler lifecycle owner id"),
    )
}
