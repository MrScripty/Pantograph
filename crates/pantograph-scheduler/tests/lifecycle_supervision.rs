use pantograph_scheduler::{
    SchedulerContractError, SchedulerLifecycleComponent, SchedulerLifecycleComponentState,
    SchedulerLifecycleOwnerDiagnostic, SchedulerLifecycleOwnerDiagnosticCode,
    SchedulerLifecycleOwnerDiagnosticSeverity, SchedulerLifecycleOwnerSnapshot,
    SchedulerLifecyclePanicState, SchedulerLifecycleQueueBound,
    ValidatedSchedulerLifecycleOwnerSnapshot, SCHEDULER_LIFECYCLE_SUPERVISION_CONTRACT_VERSION,
};

#[test]
fn valid_lifecycle_owner_fixture_decodes_and_validates() {
    let snapshot: SchedulerLifecycleOwnerSnapshot =
        serde_json::from_str(include_str!("fixtures/lifecycle_owner_running.json"))
            .expect("fixture must match scheduler lifecycle supervision contract");

    let validated = ValidatedSchedulerLifecycleOwnerSnapshot::try_from(snapshot)
        .expect("fixture must validate before composition roots consume it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_LIFECYCLE_SUPERVISION_CONTRACT_VERSION
    );
    assert_eq!(validated.as_ref().components.len(), 6);
}

#[test]
fn rejects_path_and_runtime_internal_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "owner_id": "scheduler.lifecycle.main",
        "model_path": "/models/juggernaut",
        "runtime_process_id": 1234,
        "components": []
    });

    let error = serde_json::from_value::<SchedulerLifecycleOwnerSnapshot>(value)
        .expect_err("lifecycle owner must reject path and runtime-internal fields");

    assert!(
        error.to_string().contains("unknown field `model_path`")
            || error
                .to_string()
                .contains("unknown field `runtime_process_id`"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_missing_required_component() {
    let mut snapshot: SchedulerLifecycleOwnerSnapshot =
        serde_json::from_str(include_str!("fixtures/lifecycle_owner_running.json"))
            .expect("fixture must decode");
    snapshot.components.retain(|component| {
        component.component != SchedulerLifecycleComponent::RuntimeHostDispatch
    });

    let error = ValidatedSchedulerLifecycleOwnerSnapshot::try_from(snapshot)
        .expect_err("all canonical lifecycle components must be owned");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "components.runtime_host_dispatch"
        }
    );
}

#[test]
fn rejects_duplicate_components() {
    let mut snapshot: SchedulerLifecycleOwnerSnapshot =
        serde_json::from_str(include_str!("fixtures/lifecycle_owner_running.json"))
            .expect("fixture must decode");
    let duplicate = snapshot.components[0].clone();
    snapshot.components.push(duplicate);

    let error = ValidatedSchedulerLifecycleOwnerSnapshot::try_from(snapshot)
        .expect_err("one lifecycle owner must not duplicate component ownership");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "components",
            reason: "scheduler lifecycle components must not be duplicated"
        }
    );
}

#[test]
fn queue_based_components_require_bounded_queue() {
    let mut snapshot: SchedulerLifecycleOwnerSnapshot =
        serde_json::from_str(include_str!("fixtures/lifecycle_owner_running.json"))
            .expect("fixture must decode");
    let queue_worker = snapshot
        .components
        .iter_mut()
        .find(|component| component.component == SchedulerLifecycleComponent::QueueWorker)
        .expect("fixture must include queue worker");
    queue_worker.queue_bound = None;

    let error = ValidatedSchedulerLifecycleOwnerSnapshot::try_from(snapshot)
        .expect_err("queue worker lifecycle must declare bounded queue limits");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "component.queue_bound"
        }
    );
}

#[test]
fn rejects_zero_queue_bounds() {
    let mut snapshot: SchedulerLifecycleOwnerSnapshot =
        serde_json::from_str(include_str!("fixtures/lifecycle_owner_running.json"))
            .expect("fixture must decode");
    let queue_worker = snapshot
        .components
        .iter_mut()
        .find(|component| component.component == SchedulerLifecycleComponent::QueueWorker)
        .expect("fixture must include queue worker");
    queue_worker.queue_bound = Some(SchedulerLifecycleQueueBound {
        max_in_flight: 0,
        max_buffered: 256,
    });

    let error = ValidatedSchedulerLifecycleOwnerSnapshot::try_from(snapshot)
        .expect_err("queue bounds must be finite positive values");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "queue_bound.max_in_flight",
            reason: "lifecycle queue bounds must be greater than zero"
        }
    );
}

#[test]
fn failed_or_panicked_components_require_diagnostics() {
    let mut snapshot: SchedulerLifecycleOwnerSnapshot =
        serde_json::from_str(include_str!("fixtures/lifecycle_owner_running.json"))
            .expect("fixture must decode");
    let queue_worker = snapshot
        .components
        .iter_mut()
        .find(|component| component.component == SchedulerLifecycleComponent::QueueWorker)
        .expect("fixture must include queue worker");
    queue_worker.state = SchedulerLifecycleComponentState::Failed;
    queue_worker.panic_state = SchedulerLifecyclePanicState::Observed;

    let error = ValidatedSchedulerLifecycleOwnerSnapshot::try_from(snapshot)
        .expect_err("failed lifecycle components must explain the failure");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "component.diagnostics"
        }
    );
}

#[test]
fn failed_component_accepts_typed_diagnostics() {
    let mut snapshot: SchedulerLifecycleOwnerSnapshot =
        serde_json::from_str(include_str!("fixtures/lifecycle_owner_running.json"))
            .expect("fixture must decode");
    let queue_worker = snapshot
        .components
        .iter_mut()
        .find(|component| component.component == SchedulerLifecycleComponent::QueueWorker)
        .expect("fixture must include queue worker");
    queue_worker.state = SchedulerLifecycleComponentState::Failed;
    queue_worker.panic_state = SchedulerLifecyclePanicState::Observed;
    queue_worker
        .diagnostics
        .push(SchedulerLifecycleOwnerDiagnostic {
            severity: SchedulerLifecycleOwnerDiagnosticSeverity::Error,
            code: SchedulerLifecycleOwnerDiagnosticCode::ComponentPanicObserved,
            message: "Queue worker panic was observed by the lifecycle owner.".to_string(),
            hint: Some(
                "Shutdown will drain queues before restart policy is evaluated.".to_string(),
            ),
        });

    let _validated = ValidatedSchedulerLifecycleOwnerSnapshot::try_from(snapshot)
        .expect("failed component with typed diagnostics should validate");
}
