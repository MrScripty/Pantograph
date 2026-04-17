//! Backend-owned runtime-registry lifecycle orchestration helpers.
//!
//! This module owns sync, snapshot, stop-all, restore, and reclaim sequencing
//! so runtime-registry lifecycle coordination stays separate from observation
//! translation and execution-path override reconciliation.

use async_trait::async_trait;

use crate::runtime_health::{RuntimeHealthAssessment, RuntimeHealthAssessmentSnapshot};
use crate::runtime_registry::HostRuntimeProducer;
use crate::HostRuntimeModeSnapshot;
use pantograph_runtime_registry::{
    RuntimeReclaimDisposition, RuntimeRegistry, RuntimeRegistryError,
    RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot,
};

#[async_trait]
pub trait HostRuntimeRegistryController {
    async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot;
    async fn stop_runtime_producer(&self, producer: HostRuntimeProducer);
    async fn runtime_health_assessment_snapshot(&self) -> RuntimeHealthAssessmentSnapshot {
        RuntimeHealthAssessmentSnapshot::default()
    }
}

#[async_trait]
pub trait HostRuntimeRegistryLifecycleController: HostRuntimeRegistryController {
    async fn stop_all_runtime_producers(&self);
    async fn restore_runtime(
        &self,
        restore_config: Option<inference::BackendConfig>,
    ) -> Result<(), inference::GatewayError>;
}

pub async fn sync_runtime_registry<C: HostRuntimeRegistryController + Sync>(
    controller: &C,
    registry: &RuntimeRegistry,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    let mode_info = controller.mode_info_snapshot().await;
    let health_assessments = controller.runtime_health_assessment_snapshot().await;
    crate::runtime_registry::reconcile_runtime_registry_mode_info_with_health_snapshot(
        registry,
        &mode_info,
        &health_assessments,
    )
}

pub async fn sync_runtime_registry_with_active_health_assessment<
    C: HostRuntimeRegistryController + Sync,
>(
    controller: &C,
    registry: &RuntimeRegistry,
    assessment: Option<&RuntimeHealthAssessment>,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    sync_runtime_registry_with_health_assessments(controller, registry, assessment, None).await
}

pub async fn sync_runtime_registry_with_health_assessments<
    C: HostRuntimeRegistryController + Sync,
>(
    controller: &C,
    registry: &RuntimeRegistry,
    active_assessment: Option<&RuntimeHealthAssessment>,
    embedding_assessment: Option<&RuntimeHealthAssessment>,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    let mode_info = controller.mode_info_snapshot().await;
    crate::runtime_registry::reconcile_runtime_registry_mode_info_with_health_assessments(
        registry,
        &mode_info,
        active_assessment,
        embedding_assessment,
    )
}

pub async fn runtime_registry_snapshot<C: HostRuntimeRegistryController + Sync>(
    controller: &C,
    registry: &RuntimeRegistry,
) -> RuntimeRegistrySnapshot {
    sync_runtime_registry(controller, registry).await;
    registry.snapshot()
}

pub async fn stop_all_runtime_producers_and_reconcile_runtime_registry<
    C: HostRuntimeRegistryLifecycleController + Sync,
>(
    controller: &C,
    registry: &RuntimeRegistry,
) {
    controller.stop_all_runtime_producers().await;
    sync_runtime_registry(controller, registry).await;
}

pub async fn restore_runtime_and_reconcile_runtime_registry<
    C: HostRuntimeRegistryLifecycleController + Sync,
>(
    controller: &C,
    registry: &RuntimeRegistry,
    restore_config: Option<inference::BackendConfig>,
) -> Result<(), inference::GatewayError> {
    let result = controller.restore_runtime(restore_config).await;
    sync_runtime_registry(controller, registry).await;
    result
}

pub async fn reclaim_runtime_and_reconcile_runtime_registry<
    C: HostRuntimeRegistryController + Sync,
>(
    controller: &C,
    registry: &RuntimeRegistry,
    runtime_id: &str,
) -> Result<RuntimeReclaimDisposition, RuntimeRegistryError> {
    let mode_info = controller.mode_info_snapshot().await;
    crate::runtime_registry::reconcile_runtime_registry_mode_info(registry, &mode_info);
    let live_producer = crate::runtime_registry::live_host_runtime_producer(&mode_info, runtime_id);
    let reclaim = registry.reclaim_runtime(runtime_id, live_producer.is_some())?;

    if reclaim.action == pantograph_runtime_registry::RuntimeReclaimAction::StopProducer {
        if let Some(producer) = live_producer {
            controller.stop_runtime_producer(producer).await;
        }
    }

    let mode_info = controller.mode_info_snapshot().await;
    crate::runtime_registry::reconcile_runtime_registry_mode_info(registry, &mode_info);
    Ok(reclaim)
}
