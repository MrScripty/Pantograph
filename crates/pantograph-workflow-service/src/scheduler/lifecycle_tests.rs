use crate::scheduler::lifecycle::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistry,
    WorkflowSchedulerLifecycleComponentState, WorkflowSchedulerLifecycleOwnerId,
};

#[test]
fn lifecycle_registry_owns_every_required_component_as_explicit_not_started() {
    let registry = lifecycle_registry();
    let records = registry
        .required_component_records()
        .expect("required lifecycle components");

    assert_eq!(
        registry.owner_id().as_str(),
        "workflow-service.scheduler-lifecycle.test"
    );
    assert_eq!(
        records.len(),
        WorkflowSchedulerLifecycleComponentKind::required_components().len()
    );
    for component in WorkflowSchedulerLifecycleComponentKind::required_components() {
        let record = registry.component(*component).expect("component record");
        assert_eq!(
            record.owner_id.as_str(),
            "workflow-service.scheduler-lifecycle.test"
        );
        assert_eq!(record.component, *component);
        assert_eq!(
            record.state,
            WorkflowSchedulerLifecycleComponentState::NotStarted
        );
    }
}

#[test]
fn lifecycle_registry_updates_only_owned_component_state() {
    let mut registry = lifecycle_registry();

    let updated = registry
        .update_component_state(
            WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch,
            WorkflowSchedulerLifecycleComponentState::Running,
        )
        .expect("update runtime-host dispatch state");

    assert_eq!(
        updated.component,
        WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch
    );
    assert_eq!(
        updated.state,
        WorkflowSchedulerLifecycleComponentState::Running
    );
    assert_eq!(
        registry
            .component(WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch)
            .expect("runtime-host dispatch component")
            .state,
        WorkflowSchedulerLifecycleComponentState::Running
    );
    assert_eq!(
        registry
            .component(WorkflowSchedulerLifecycleComponentKind::QueueWorker)
            .expect("queue worker component")
            .state,
        WorkflowSchedulerLifecycleComponentState::NotStarted
    );

    registry
        .update_component_state(
            WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch,
            WorkflowSchedulerLifecycleComponentState::ShuttingDown,
        )
        .expect("update runtime-host dispatch shutdown state");
    let shutdown = registry
        .update_component_state(
            WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch,
            WorkflowSchedulerLifecycleComponentState::Shutdown,
        )
        .expect("update runtime-host dispatch final shutdown state");

    assert_eq!(
        shutdown.state,
        WorkflowSchedulerLifecycleComponentState::Shutdown
    );
}

#[test]
fn lifecycle_component_kinds_have_stable_snapshot_names() {
    let names: Vec<_> = WorkflowSchedulerLifecycleComponentKind::required_components()
        .iter()
        .map(|component| component.as_str())
        .collect();

    assert_eq!(
        names,
        vec![
            "queue_worker",
            "task_execution_worker",
            "dependency_readiness_action",
            "resource_observation_loop",
            "runtime_host_dispatch",
            "retry_loop",
            "reservation_cleanup",
        ]
    );
}

#[test]
fn lifecycle_owner_id_rejects_blank_value() {
    let error =
        WorkflowSchedulerLifecycleOwnerId::parse(" ").expect_err("blank owner id must fail");

    assert!(error.to_string().contains("InvalidLifecycleOwnerId"));
}

fn lifecycle_registry() -> WorkflowSchedulerLifecycleComponentRegistry {
    WorkflowSchedulerLifecycleComponentRegistry::new(
        WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.scheduler-lifecycle.test")
            .expect("owner id"),
    )
}
