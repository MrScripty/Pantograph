//! Backend-owned runtime-registry lifecycle orchestration helpers.
//!
//! This module owns sync, snapshot, stop-all, restore, and reclaim sequencing
//! so runtime-registry lifecycle coordination stays separate from observation
//! translation and execution-path override reconciliation.

use std::future::Future;
use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;

use crate::runtime_health::{RuntimeHealthAssessment, RuntimeHealthAssessmentSnapshot};
use crate::runtime_registry::HostRuntimeProducer;
use crate::HostRuntimeModeSnapshot;
use pantograph_runtime_registry::{
    RuntimeReclaimDisposition, RuntimeRegistry, RuntimeRegistryError,
    RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot, RuntimeRetentionDecision,
    RuntimeRetentionDisposition, RuntimeTransition, RuntimeWarmupDecision,
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

#[derive(Debug, Error)]
pub enum RuntimeWarmupCoordinationError {
    #[error(transparent)]
    Registry(#[from] RuntimeRegistryError),
    #[error(
        "timed out waiting for runtime '{runtime_id}' to finish warmup or shutdown transition"
    )]
    Timeout { runtime_id: String },
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

pub async fn run_runtime_transition_and_reconcile_runtime_registry<C, F, Fut, T, E>(
    controller: &C,
    registry: &RuntimeRegistry,
    transition: F,
) -> Result<T, E>
where
    C: HostRuntimeRegistryController + Sync,
    F: FnOnce(&C) -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let result = transition(controller).await;
    sync_runtime_registry(controller, registry).await;
    result
}

pub async fn consume_active_runtime_warmup_disposition<C: HostRuntimeRegistryController + Sync>(
    controller: &C,
    registry: &RuntimeRegistry,
    runtime_id: &str,
    poll_interval: Duration,
    wait_timeout: Duration,
) -> Result<(), RuntimeWarmupCoordinationError> {
    match registry.warmup_disposition(runtime_id)?.decision {
        RuntimeWarmupDecision::ReuseLoadedRuntime => Ok(()),
        RuntimeWarmupDecision::StartRuntime => {
            let mode_info = reconcile_active_runtime_mode_info_snapshot(controller, registry).await;

            match registry.warmup_disposition(runtime_id)?.decision {
                RuntimeWarmupDecision::ReuseLoadedRuntime => Ok(()),
                RuntimeWarmupDecision::WaitForTransition => {
                    wait_for_active_runtime_warmup_transition(
                        controller,
                        registry,
                        runtime_id,
                        poll_interval,
                        wait_timeout,
                    )
                    .await
                }
                RuntimeWarmupDecision::StartRuntime => {
                    mark_runtime_warmup_started(registry, runtime_id, &mode_info)
                }
            }
        }
        RuntimeWarmupDecision::WaitForTransition => {
            wait_for_active_runtime_warmup_transition(
                controller,
                registry,
                runtime_id,
                poll_interval,
                wait_timeout,
            )
            .await
        }
    }
}

async fn wait_for_active_runtime_warmup_transition<C: HostRuntimeRegistryController + Sync>(
    controller: &C,
    registry: &RuntimeRegistry,
    runtime_id: &str,
    poll_interval: Duration,
    wait_timeout: Duration,
) -> Result<(), RuntimeWarmupCoordinationError> {
    let wait_future = async {
        loop {
            let mode_info = reconcile_active_runtime_mode_info_snapshot(controller, registry).await;

            match registry.warmup_disposition(runtime_id)?.decision {
                RuntimeWarmupDecision::ReuseLoadedRuntime => return Ok(()),
                RuntimeWarmupDecision::StartRuntime => {
                    return mark_runtime_warmup_started(registry, runtime_id, &mode_info);
                }
                RuntimeWarmupDecision::WaitForTransition => {
                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    };

    tokio::time::timeout(wait_timeout, wait_future)
        .await
        .map_err(|_| RuntimeWarmupCoordinationError::Timeout {
            runtime_id: runtime_id.to_string(),
        })?
}

async fn reconcile_active_runtime_mode_info_snapshot<C: HostRuntimeRegistryController + Sync>(
    controller: &C,
    registry: &RuntimeRegistry,
) -> HostRuntimeModeSnapshot {
    let mode_info = controller.mode_info_snapshot().await;
    crate::runtime_registry::reconcile_active_runtime_mode_info(registry, &mode_info, false);
    mode_info
}

fn mark_runtime_warmup_started(
    registry: &RuntimeRegistry,
    runtime_id: &str,
    mode_info: &HostRuntimeModeSnapshot,
) -> Result<(), RuntimeWarmupCoordinationError> {
    registry
        .transition_runtime(
            runtime_id,
            RuntimeTransition::WarmupStarted {
                runtime_instance_id: mode_info
                    .active_runtime
                    .as_ref()
                    .and_then(|snapshot| snapshot.runtime_instance_id.clone()),
            },
        )
        .map(|_| ())
        .map_err(Into::into)
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

pub async fn release_reservation_and_reconcile_runtime_registry<
    C: HostRuntimeRegistryController + Sync,
>(
    controller: &C,
    registry: &RuntimeRegistry,
    reservation_id: u64,
) -> Result<Option<RuntimeRetentionDisposition>, RuntimeRegistryError> {
    let disposition = registry.release_reservation_if_present(reservation_id)?;

    if let Some(disposition) = disposition.as_ref() {
        if disposition.decision == RuntimeRetentionDecision::Evict {
            reclaim_runtime_and_reconcile_runtime_registry(
                controller,
                registry,
                &disposition.runtime_id,
            )
            .await?;
        }
    }

    Ok(disposition)
}
