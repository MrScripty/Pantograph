use super::*;

#[test]
fn register_runtime_canonicalizes_runtime_and_backend_keys() {
    let registry = RuntimeRegistry::new();

    let snapshot = registry.register_runtime(
        RuntimeRegistration::new("PyTorch", "PyTorch sidecar").with_backend_keys(vec![
            "torch".to_string(),
            "PyTorch".to_string(),
            "torch".to_string(),
        ]),
    );

    assert_eq!(snapshot.runtime_id, "pytorch");
    assert_eq!(snapshot.backend_keys, vec!["pytorch".to_string()]);
    assert_eq!(snapshot.status, RuntimeRegistryStatus::Stopped);
}

#[test]
fn transition_runtime_rejects_invalid_state_changes() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));

    let err = registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Busy {
                runtime_instance_id: Some("runtime-1".to_string()),
            },
        )
        .expect_err("busy from stopped should be rejected");

    assert_eq!(
        err,
        RuntimeRegistryError::InvalidTransition {
            runtime_id: "llama_cpp".to_string(),
            from: RuntimeRegistryStatus::Stopped,
            to: RuntimeRegistryStatus::Busy,
        }
    );
}

#[test]
fn reservation_lifecycle_updates_snapshot_state() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("onnxruntime", "ONNX Runtime")
            .with_backend_keys(vec!["onnxruntime".to_string()]),
    );
    registry
        .transition_runtime(
            "onnx-runtime",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("python-runtime:onnx-runtime".to_string()),
            },
        )
        .expect("ready transition");

    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "onnxruntime".to_string(),
            workflow_id: "wf-1".to_string(),
            reservation_owner_id: None,
            usage_profile: Some("audio".to_string()),
            model_id: Some("model-a".to_string()),
            pin_runtime: true,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .expect("acquire reservation");

    let snapshot = registry.snapshot();
    assert_eq!(snapshot.runtimes.len(), 1);
    assert_eq!(snapshot.runtimes[0].runtime_id, "onnx-runtime");
    assert_eq!(
        snapshot.runtimes[0].active_reservation_ids,
        vec![lease.reservation_id]
    );
    assert_eq!(snapshot.reservations.len(), 1);
    assert_eq!(snapshot.reservations[0].runtime_id, "onnx-runtime");
    assert_eq!(
        snapshot.reservations[0].retention_hint,
        RuntimeRetentionHint::KeepAlive
    );
    assert_eq!(
        registry
            .retention_disposition("onnxruntime")
            .expect("disposition"),
        RuntimeRetentionDisposition::retain(
            "onnx-runtime",
            RuntimeRetentionReason::KeepAliveReservation,
        )
    );

    let disposition = registry
        .release_reservation_with_disposition(lease.reservation_id)
        .expect("release reservation");
    assert_eq!(
        disposition,
        RuntimeRetentionDisposition::evict("onnx-runtime")
    );

    let released = registry.snapshot();
    assert!(released.runtimes[0].active_reservation_ids.is_empty());
    assert!(released.reservations.is_empty());
}

#[test]
fn register_runtime_without_new_budget_preserves_existing_budget() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(
        RuntimeRegistration::new("llama.cpp", "llama.cpp").with_admission_budget(
            RuntimeAdmissionBudget::new(Some(4096), Some(8192))
                .with_safety_margin_ram_mb(256)
                .with_safety_margin_vram_mb(1024),
        ),
    );

    registry.register_runtime(
        RuntimeRegistration::new("llama.cpp", "llama.cpp")
            .with_backend_keys(vec!["llama_cpp".to_string()]),
    );
    registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime-1".to_string()),
            },
        )
        .expect("ready transition");

    let err = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "wf-budget".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements {
                estimated_peak_vram_mb: Some(7200),
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: Some(4096),
                estimated_min_ram_mb: None,
            }),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("preserved vram budget should still reject oversized request");

    assert_eq!(
        err,
        RuntimeRegistryError::AdmissionRejected {
            runtime_id: "llama_cpp".to_string(),
            failure: RuntimeAdmissionFailure::InsufficientVram {
                requested_mb: 7200,
                available_mb: 7168,
                reserved_mb: 0,
                total_mb: 8192,
                safety_margin_mb: 1024,
            },
        }
    );
}

#[test]
fn reservations_are_rejected_while_runtime_is_stopping() {
    let registry = RuntimeRegistry::new();
    registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
    registry
        .transition_runtime(
            "llama.cpp",
            RuntimeTransition::Ready {
                runtime_instance_id: Some("runtime-llama.cpp".to_string()),
            },
        )
        .expect("ready transition");
    registry
        .transition_runtime("llama.cpp", RuntimeTransition::StopRequested)
        .expect("stop transition");

    let err = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "llama.cpp".to_string(),
            workflow_id: "wf-stop".to_string(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect_err("stopping runtime should reject reservations");

    assert_eq!(
        err,
        RuntimeRegistryError::ReservationRejected("llama_cpp".to_string())
    );
}
