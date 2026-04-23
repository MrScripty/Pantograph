use super::*;

#[test]
fn release_reservation_if_present_is_idempotent() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "idempotent-runtime".to_string(),
        display_name: "idempotent-runtime".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("idempotent-runtime-1".to_string()),
        last_error: None,
    }]);

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "idempotent-runtime".to_string(),
            workflow_id: "wf-idempotent".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");

    assert_eq!(
        registry
            .release_reservation_if_present(lease.reservation_id)
            .expect("first release"),
        Some(RuntimeRetentionDisposition::evict("idempotent-runtime"))
    );
    assert_eq!(
        registry
            .release_reservation_if_present(lease.reservation_id)
            .expect("second release"),
        None
    );
}

#[test]
fn reclaim_runtime_marks_inactive_evictable_runtime_stopped() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "inactive-reclaim".to_string(),
        display_name: "inactive-reclaim".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("inactive-reclaim-1".to_string()),
        last_error: None,
    }]);

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "inactive-reclaim".to_string(),
            workflow_id: "wf-inactive".to_string(),
            reservation_owner_id: Some("session-inactive".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");
    registry
        .release_reservation_if_present(lease.reservation_id)
        .expect("release");

    assert_eq!(
        registry
            .reclaim_runtime("inactive-reclaim", false)
            .expect("reclaim disposition"),
        RuntimeReclaimDisposition::no_action(
            "inactive-reclaim",
            RuntimeRetentionReason::Evictable,
            RuntimeRegistryStatus::Stopped,
        )
    );

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopped);
    assert!(snapshot.runtimes[0].runtime_instance_id.is_none());
    assert!(snapshot.runtimes[0].models.is_empty());
}

#[test]
fn reclaim_runtime_requests_stop_for_active_evictable_runtime() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "active-reclaim".to_string(),
        display_name: "active-reclaim".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("active-reclaim-1".to_string()),
        last_error: None,
    }]);

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "active-reclaim".to_string(),
            workflow_id: "wf-active".to_string(),
            reservation_owner_id: Some("session-active".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");
    registry
        .release_reservation_if_present(lease.reservation_id)
        .expect("release");

    assert_eq!(
        registry
            .reclaim_runtime("active-reclaim", true)
            .expect("reclaim disposition"),
        RuntimeReclaimDisposition::stop_producer("active-reclaim", RuntimeRegistryStatus::Stopping,)
    );

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopping);
    assert_eq!(
        snapshot.runtimes[0].runtime_instance_id.as_deref(),
        Some("active-reclaim-1")
    );
}

#[test]
fn reclaim_runtime_keeps_keep_alive_runtime_retained() {
    let registry = RuntimeRegistry::new();
    registry.observe_runtimes(vec![RuntimeObservation {
        runtime_id: "retained-reclaim".to_string(),
        display_name: "retained-reclaim".to_string(),
        backend_keys: vec!["llama_cpp".to_string()],
        model_id: Some("model-a".to_string()),
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some("retained-reclaim-1".to_string()),
        last_error: None,
    }]);

    let _lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "retained-reclaim".to_string(),
            workflow_id: "wf-retained".to_string(),
            reservation_owner_id: Some("session-retained".to_string()),
            usage_profile: None,
            model_id: Some("model-a".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("reservation");

    assert_eq!(
        registry
            .reclaim_runtime("retained-reclaim", true)
            .expect("reclaim disposition"),
        RuntimeReclaimDisposition::no_action(
            "retained-reclaim",
            RuntimeRetentionReason::KeepAliveReservation,
            RuntimeRegistryStatus::Ready,
        )
    );

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Ready);
}
