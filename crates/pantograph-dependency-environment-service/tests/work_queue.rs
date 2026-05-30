use pantograph_dependency_environment_service::{
    DependencyEnvironmentProvider, DependencyEnvironmentReadinessSnapshotProvider,
    DependencyReadinessDiagnosticContext, DependencyReadinessFreshnessPolicy,
    DependencyReadinessTaskId, DependencyReadinessWorkItem, DependencyReadinessWorkItemProvenance,
    DependencyReadinessWorkQueue, DependencyReadinessWorkQueueError,
    DependencyReadinessWorkQueueEvent, DependencyReadinessWorkflowRunId,
    DependencyReadinessWorkflowSessionId,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentReadinessState, ValidatedDependencyEnvironmentRequest,
};

const RESOLVE_REQUEST: &str = include_str!(
    "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_resolve_request.json"
);

#[test]
fn work_queue_enqueues_fifo_items_and_replaces_duplicate_task_request() {
    let queue = DependencyReadinessWorkQueue::new();
    let first = work_item("session.001", "run.001", "task.image");
    let second = work_item("session.001", "run.001", "task.upscale");
    let replacement = work_item("session.001", "run.001", "task.image")
        .with_diagnostic_context(
            DependencyReadinessDiagnosticContext::parse("retry after stale snapshot")
                .expect("diagnostic context"),
        )
        .with_freshness_policy(
            DependencyReadinessFreshnessPolicy::new(Some(1_770_000_000_000), 5)
                .expect("freshness policy"),
        );

    assert_eq!(
        queue.enqueue(first),
        DependencyReadinessWorkQueueEvent::Enqueued
    );
    assert_eq!(
        queue.enqueue(second.clone()),
        DependencyReadinessWorkQueueEvent::Enqueued
    );
    assert_eq!(
        queue.enqueue(replacement.clone()),
        DependencyReadinessWorkQueueEvent::Replaced
    );
    assert_eq!(queue.len(), 2);

    let popped = queue.pop_next().expect("first queued item");
    assert_eq!(popped.provenance.task_id.as_str(), "task.image");
    assert_eq!(
        popped
            .diagnostic_context
            .as_ref()
            .map(DependencyReadinessDiagnosticContext::as_str),
        Some("retry after stale snapshot")
    );
    assert_eq!(popped.freshness_policy.max_attempts, 5);

    assert_eq!(queue.pop_next(), Some(second));
    assert!(queue.is_empty());
}

#[test]
fn work_queue_dedupe_key_keeps_task_provenance_distinct() {
    let queue = DependencyReadinessWorkQueue::new();

    assert_eq!(
        queue.enqueue(work_item("session.001", "run.001", "task.image")),
        DependencyReadinessWorkQueueEvent::Enqueued
    );
    assert_eq!(
        queue.enqueue(work_item("session.001", "run.001", "task.other")),
        DependencyReadinessWorkQueueEvent::Enqueued
    );
    assert_eq!(
        queue.enqueue(work_item("session.002", "run.001", "task.image")),
        DependencyReadinessWorkQueueEvent::Enqueued
    );

    assert_eq!(queue.len(), 3);
}

#[test]
fn work_queue_rejects_invalid_provenance_policy_and_diagnostic_context() {
    assert!(matches!(
        DependencyReadinessWorkflowSessionId::parse(""),
        Err(DependencyReadinessWorkQueueError::InvalidField {
            field: "dependency_readiness_work_item.session_id",
            ..
        })
    ));
    assert!(matches!(
        DependencyReadinessFreshnessPolicy::new(None, 0),
        Err(DependencyReadinessWorkQueueError::InvalidField {
            field: "dependency_readiness_work_item.max_attempts",
            ..
        })
    ));
    assert!(matches!(
        DependencyReadinessDiagnosticContext::parse("bad\ncontext"),
        Err(DependencyReadinessWorkQueueError::InvalidField {
            field: "dependency_readiness_work_item.diagnostic_context",
            ..
        })
    ));
}

#[test]
fn provider_miss_does_not_record_readiness_work() {
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let queue = DependencyReadinessWorkQueue::new();
    let request = validated_request();

    let result = provider.check(&request);

    assert_eq!(
        result.readiness_state,
        DependencyEnvironmentReadinessState::Missing
    );
    assert_eq!(provider.snapshot_count(), 0);
    assert!(queue.is_empty());
}

fn work_item(
    session_id: &str,
    workflow_run_id: &str,
    task_id: &str,
) -> DependencyReadinessWorkItem {
    DependencyReadinessWorkItem::new(
        DependencyReadinessWorkItemProvenance::new(
            DependencyReadinessWorkflowSessionId::parse(session_id).expect("session id"),
            DependencyReadinessWorkflowRunId::parse(workflow_run_id).expect("workflow run id"),
            DependencyReadinessTaskId::parse(task_id).expect("task id"),
        ),
        validated_request(),
    )
}

fn validated_request() -> ValidatedDependencyEnvironmentRequest {
    let value: serde_json::Value =
        serde_json::from_str(RESOLVE_REQUEST).expect("request fixture should parse");
    ValidatedDependencyEnvironmentRequest::try_from(value).expect("request fixture should validate")
}
