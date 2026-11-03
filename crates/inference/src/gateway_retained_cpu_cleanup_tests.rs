use super::*;
use crate::{CandleCpuCalibrationConfig, InferenceExecutionCancellationHandle};
use pantograph_runtime_registry::{
    RuntimeModelResidentEstimate, RuntimeReservationRequest, RuntimeReservationRequirements,
    RuntimeReservationResourceClaim, RuntimeRetentionDecision, RuntimeRetentionHint,
};
use std::time::Duration;

fn gateway() -> InferenceGateway {
    InferenceGateway::new_calibrated_candle_cpu(
        CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60)).unwrap(),
    )
    .unwrap()
}
fn registry(target: &crate::PumasArtifactLoadTarget) -> RuntimeRegistry {
    let registry = RuntimeRegistry::new();
    // Explicit synthetic operator estimate; this does not measure allocator RAM.
    registry
        .configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
            runtime_id: "candle".into(),
            model_id: target.local_load_path.clone(),
            requirements: RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(32 * 1024),
            ]),
        }])
        .unwrap();
    registry
}
fn lease(
    registry: &RuntimeRegistry,
    model: &str,
    owner: &str,
    hint: RuntimeRetentionHint,
) -> RuntimeReservationLease {
    registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "candle".into(),
            workflow_id: "native-cleanup".into(),
            reservation_owner_id: Some(owner.into()),
            usage_profile: None,
            model_id: Some(model.into()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(256),
            ])),
            retention_hint: hint,
        })
        .unwrap()
}
fn state(
    registry: &RuntimeRegistry,
) -> (
    Vec<pantograph_runtime_registry::RuntimeRegistryRuntimeSnapshot>,
    Vec<RuntimeReservationLease>,
) {
    let snapshot = registry.snapshot();
    (snapshot.runtimes, snapshot.reservations)
}

#[tokio::test]
async fn actual_cpu_reuse_rejects_old_epoch_then_retains_successor_and_accounting() {
    let (_directory, request, target, decision) = crate::selected_embedding_execution::fixture(8);
    let gateway = gateway();
    let registry = registry(&target);
    let task = lease(
        &registry,
        &target.model_ref.model_id,
        "task",
        RuntimeRetentionHint::Ephemeral,
    );
    let successor = lease(
        &registry,
        &target.model_ref.model_id,
        "session",
        RuntimeRetentionHint::KeepAlive,
    );
    let cold = gateway
        .execute_selected_embedding_with_cancellation(
            request.clone(),
            target.clone(),
            decision.clone(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let first = gateway
        .publish_resident_cpu_cleanup_owner(&registry)
        .await
        .unwrap();
    let loaded = gateway.cpu_calibration_instance_for_test().unwrap();
    let warm = gateway
        .execute_selected_embedding_with_cancellation(
            request.clone(),
            target.clone(),
            decision.clone(),
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    assert_eq!(cold, warm); // Actual vectors and usage survive verified reuse.
    assert_eq!(loaded, gateway.cpu_calibration_instance_for_test().unwrap());
    assert_eq!(
        gateway.runtime_lifecycle_snapshot().await.runtime_reused,
        Some(true)
    );
    let before = state(&registry);
    assert!(gateway
        .release_retained_cpu_reservation(&registry, &task, &first)
        .await
        .is_err());
    assert_eq!(before, state(&registry));
    let current = gateway
        .publish_resident_cpu_cleanup_owner(&registry)
        .await
        .unwrap();
    let accounted = state(&registry).0[0].model_resource_residency.clone();
    assert_eq!(
        gateway
            .release_retained_cpu_reservation(&registry, &task, &current)
            .await
            .unwrap()
            .decision,
        RuntimeRetentionDecision::Retain
    );
    assert_eq!(state(&registry).1, vec![successor.clone()]);
    assert_eq!(state(&registry).0[0].model_resource_residency, accounted);
    assert_eq!(loaded, gateway.cpu_calibration_instance_for_test().unwrap());
    let after_cleanup = gateway
        .execute_selected_embedding_with_cancellation(
            request,
            target,
            decision,
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    assert_eq!(warm, after_cleanup);
    assert_eq!(loaded, gateway.cpu_calibration_instance_for_test().unwrap());
    assert_eq!(
        gateway.runtime_lifecycle_snapshot().await.runtime_reused,
        Some(true)
    );
    let current = gateway
        .publish_resident_cpu_cleanup_owner(&registry)
        .await
        .unwrap();
    let before_last = state(&registry);
    assert!(gateway
        .release_retained_cpu_reservation(&registry, &successor, &current)
        .await
        .is_err());
    assert_eq!(before_last, state(&registry));
    // Ordinary stop is teardown, not qualification of last-lease serial cleanup.
    gateway.stop().await.unwrap();
    assert!(gateway.cpu_calibration_instance_for_test().is_none());
    assert!(gateway
        .publish_resident_cpu_cleanup_owner(&registry)
        .await
        .is_err());
    assert_eq!(before_last, state(&registry));
}

#[tokio::test]
async fn same_bytes_foreign_authority_and_actual_replacement_cannot_release_successor() {
    let (_directory, request, target, decision) = crate::selected_embedding_execution::fixture(8);
    let native = gateway();
    let foreign = gateway();
    let registry = registry(&target);
    let task = lease(
        &registry,
        &target.model_ref.model_id,
        "task",
        RuntimeRetentionHint::Ephemeral,
    );
    let _successor = lease(
        &registry,
        &target.model_ref.model_id,
        "next",
        RuntimeRetentionHint::KeepAlive,
    );
    for gateway in [&native, &foreign] {
        gateway
            .execute_selected_embedding_with_cancellation(
                request.clone(),
                target.clone(),
                decision.clone(),
                InferenceExecutionCancellationHandle::running(),
            )
            .await
            .unwrap();
    }
    let expected = native
        .publish_resident_cpu_cleanup_owner(&registry)
        .await
        .unwrap();
    let foreign_registry = super::tests::registry(&target);
    let foreign_stamp = foreign
        .publish_resident_cpu_cleanup_owner(&foreign_registry)
        .await
        .unwrap();
    let before = state(&registry);
    assert!(native
        .release_retained_cpu_reservation(&registry, &task, &foreign_stamp)
        .await
        .is_err());
    assert_eq!(before, state(&registry));
    let (_replacement_dir, new_request, new_target, new_decision) =
        crate::selected_embedding_execution::fixture(12);
    native
        .execute_selected_embedding_with_cancellation(
            new_request,
            new_target,
            new_decision,
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let replacement = native.cpu_calibration_instance_for_test().unwrap();
    assert!(native
        .release_retained_cpu_reservation(&registry, &task, &expected)
        .await
        .is_err());
    assert_eq!(before, state(&registry));
    assert_eq!(
        replacement,
        native.cpu_calibration_instance_for_test().unwrap()
    );
    native.stop().await.unwrap();
    foreign.stop().await.unwrap();
}

#[tokio::test]
async fn actual_publication_without_resident_estimate_cannot_release_claims() {
    let (_directory, request, target, decision) = crate::selected_embedding_execution::fixture(8);
    let gateway = gateway();
    let registry = RuntimeRegistry::new();
    gateway
        .execute_selected_embedding_with_cancellation(
            request,
            target.clone(),
            decision,
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let owner = gateway
        .publish_resident_cpu_cleanup_owner(&registry)
        .await
        .unwrap();
    let task = lease(
        &registry,
        &target.model_ref.model_id,
        "task",
        RuntimeRetentionHint::Ephemeral,
    );
    let _successor = lease(
        &registry,
        &target.model_ref.model_id,
        "session",
        RuntimeRetentionHint::KeepAlive,
    );
    let before = state(&registry);
    assert!(gateway
        .release_retained_cpu_reservation(&registry, &task, &owner)
        .await
        .is_err());
    assert_eq!(before, state(&registry));
    gateway.stop().await.unwrap();
}
