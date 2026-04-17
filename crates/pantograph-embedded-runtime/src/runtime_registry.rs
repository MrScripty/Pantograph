//! Backend-owned runtime-registry translation helpers.
//!
//! This module converts gateway lifecycle facts and producer-specific runtime
//! snapshots into `pantograph_runtime_registry::RuntimeObservation` values so
//! host adapters do not own registry-observation mapping logic.

use async_trait::async_trait;

use crate::runtime_health::{RuntimeHealthAssessment, RuntimeHealthAssessmentSnapshot};
pub use crate::runtime_registry_observations::{
    active_runtime_descriptor, active_runtime_id, active_runtime_observation,
    active_runtime_observation_with_health_assessment, embedding_runtime_id,
    embedding_runtime_observation, embedding_runtime_observation_with_health_assessment,
    live_host_runtime_producer, observations_from_mode_info,
    observations_from_mode_info_with_active_health_assessment,
    observations_from_mode_info_with_health_assessments, reconcile_active_runtime_mode_info,
    reconcile_runtime_registry_mode_info_with_health_snapshot, ActiveRuntimeDescriptor,
};
use crate::HostRuntimeModeSnapshot;
use pantograph_runtime_identity::{
    canonical_runtime_id, runtime_backend_key_aliases, runtime_display_name,
};
use pantograph_runtime_registry::{
    observed_runtime_status_from_lifecycle, RuntimeObservation, RuntimeRegistry,
    RuntimeRegistryRuntimeSnapshot, RuntimeRegistrySnapshot, RuntimeRegistryStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRuntimeProducer {
    Active,
    Embedding,
}

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

pub fn reconcile_runtime_registry_mode_info(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    reconcile_runtime_registry_mode_info_with_health_assessments(registry, mode_info, None, None)
}

pub fn reconcile_runtime_registry_mode_info_with_active_health_assessment(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
    assessment: Option<&RuntimeHealthAssessment>,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    reconcile_runtime_registry_mode_info_with_health_assessments(
        registry, mode_info, assessment, None,
    )
}

pub fn reconcile_runtime_registry_mode_info_with_health_assessments(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
    active_assessment: Option<&RuntimeHealthAssessment>,
    embedding_assessment: Option<&RuntimeHealthAssessment>,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    registry.observe_runtimes(observations_from_mode_info_with_health_assessments(
        mode_info,
        active_assessment,
        embedding_assessment,
    ))
}

pub async fn sync_runtime_registry<C: HostRuntimeRegistryController + Sync>(
    controller: &C,
    registry: &RuntimeRegistry,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    let mode_info = controller.mode_info_snapshot().await;
    let health_assessments = controller.runtime_health_assessment_snapshot().await;
    reconcile_runtime_registry_mode_info_with_health_snapshot(
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
    reconcile_runtime_registry_mode_info_with_health_assessments(
        registry,
        &mode_info,
        active_assessment,
        embedding_assessment,
    )
}

pub fn reconcile_runtime_registry_snapshot_override(
    registry: &RuntimeRegistry,
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_id: Option<&str>,
) -> Option<RuntimeRegistryRuntimeSnapshot> {
    reconcile_runtime_registry_snapshot_override_with_health_assessment(
        registry, snapshot, model_id, None,
    )
}

pub fn reconcile_runtime_registry_snapshot_override_with_health_assessment(
    registry: &RuntimeRegistry,
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_id: Option<&str>,
    assessment: Option<&RuntimeHealthAssessment>,
) -> Option<RuntimeRegistryRuntimeSnapshot> {
    let runtime_id = snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty())?;
    let display_name = runtime_display_name(&runtime_id)
        .unwrap_or(runtime_id.as_str())
        .to_string();
    let backend_keys = runtime_backend_key_aliases(&display_name, &runtime_id);

    let observation = crate::runtime_registry_observations::observation_with_health_assessment(
        RuntimeObservation {
            runtime_id,
            display_name: display_name.clone(),
            backend_keys,
            model_id: model_id.map(ToOwned::to_owned),
            status: observed_runtime_status_from_lifecycle(
                snapshot.active,
                snapshot.warmup_started_at_ms,
                snapshot.warmup_completed_at_ms,
                snapshot.last_error.is_some(),
            ),
            runtime_instance_id: snapshot.runtime_instance_id.clone(),
            last_error: snapshot.last_error.clone(),
        },
        assessment,
    );

    let observation = preserve_matching_unhealthy_runtime(registry, observation);

    Some(registry.observe_runtime(observation))
}

fn preserve_matching_unhealthy_runtime(
    registry: &RuntimeRegistry,
    mut observation: RuntimeObservation,
) -> RuntimeObservation {
    if matches!(
        observation.status,
        RuntimeRegistryStatus::Stopped | RuntimeRegistryStatus::Failed
    ) {
        return observation;
    }

    let Some(existing_runtime) = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == observation.runtime_id)
    else {
        return observation;
    };

    if existing_runtime.status != RuntimeRegistryStatus::Unhealthy {
        return observation;
    }

    if existing_runtime.runtime_instance_id != observation.runtime_instance_id {
        return observation;
    }

    observation.status = RuntimeRegistryStatus::Unhealthy;
    if observation.last_error.is_none() {
        observation.last_error = existing_runtime.last_error;
    }

    observation
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
) -> Result<
    pantograph_runtime_registry::RuntimeReclaimDisposition,
    pantograph_runtime_registry::RuntimeRegistryError,
> {
    let mode_info = controller.mode_info_snapshot().await;
    reconcile_runtime_registry_mode_info(registry, &mode_info);
    let live_producer = live_host_runtime_producer(&mode_info, runtime_id);
    let reclaim = registry.reclaim_runtime(runtime_id, live_producer.is_some())?;

    if reclaim.action == pantograph_runtime_registry::RuntimeReclaimAction::StopProducer {
        if let Some(producer) = live_producer {
            controller.stop_runtime_producer(producer).await;
        }
    }

    let mode_info = controller.mode_info_snapshot().await;
    reconcile_runtime_registry_mode_info(registry, &mode_info);
    Ok(reclaim)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::runtime_health::{RuntimeHealthAssessment, RuntimeHealthState};
    use pantograph_runtime_registry::{
        RuntimeReclaimDisposition, RuntimeRegistration, RuntimeRegistryStatus,
        RuntimeRetentionReason,
    };

    struct MockHostRuntimeController {
        mode_info: Mutex<HostRuntimeModeSnapshot>,
        stopped_producers: Mutex<Vec<HostRuntimeProducer>>,
        stop_all_calls: Mutex<u32>,
        restore_calls: Mutex<Vec<Option<inference::BackendConfig>>>,
        restore_should_fail: Mutex<bool>,
    }

    #[async_trait]
    impl HostRuntimeRegistryController for MockHostRuntimeController {
        async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
            self.mode_info
                .lock()
                .expect("mode info lock poisoned")
                .clone()
        }

        async fn stop_runtime_producer(&self, producer: HostRuntimeProducer) {
            self.stopped_producers
                .lock()
                .expect("stopped producers lock poisoned")
                .push(producer);
            let mut mode_info = self.mode_info.lock().expect("mode info lock poisoned");
            match producer {
                HostRuntimeProducer::Active => {
                    mode_info.active_runtime = Some(inference::RuntimeLifecycleSnapshot::default());
                }
                HostRuntimeProducer::Embedding => {
                    mode_info.embedding_runtime =
                        Some(inference::RuntimeLifecycleSnapshot::default());
                }
            }
        }
    }

    #[async_trait]
    impl HostRuntimeRegistryLifecycleController for MockHostRuntimeController {
        async fn stop_all_runtime_producers(&self) {
            *self
                .stop_all_calls
                .lock()
                .expect("stop-all calls lock poisoned") += 1;
            let mut mode_info = self.mode_info.lock().expect("mode info lock poisoned");
            mode_info.active_runtime = Some(inference::RuntimeLifecycleSnapshot::default());
            mode_info.embedding_runtime = Some(inference::RuntimeLifecycleSnapshot::default());
        }

        async fn restore_runtime(
            &self,
            restore_config: Option<inference::BackendConfig>,
        ) -> Result<(), inference::GatewayError> {
            self.restore_calls
                .lock()
                .expect("restore calls lock poisoned")
                .push(restore_config);
            if *self
                .restore_should_fail
                .lock()
                .expect("restore failure flag lock poisoned")
            {
                return Err(inference::GatewayError::NoBackend);
            }
            let mut mode_info = self.mode_info.lock().expect("mode info lock poisoned");
            mode_info.active_runtime = Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-restored".to_string()),
                warmup_started_at_ms: Some(30),
                warmup_completed_at_ms: Some(40),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            });
            mode_info.embedding_runtime = Some(inference::RuntimeLifecycleSnapshot::default());
            Ok(())
        }
    }

    #[test]
    fn live_host_runtime_producer_matches_active_runtime_aliases() {
        let producer = live_host_runtime_producer(
            &HostRuntimeModeSnapshot {
                backend_name: Some("PyTorch".to_string()),
                backend_key: Some("pytorch".to_string()),
                active_model_target: None,
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("pytorch".to_string()),
                    runtime_instance_id: Some("torch-main-1".to_string()),
                    warmup_started_at_ms: None,
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
            "PyTorch",
        );

        assert_eq!(producer, Some(HostRuntimeProducer::Active));
    }

    #[test]
    fn live_host_runtime_producer_matches_embedding_runtime_aliases() {
        let producer = live_host_runtime_producer(
            &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: None,
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot::default()),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp.embedding".to_string()),
                    runtime_instance_id: Some("llama-embed-1".to_string()),
                    warmup_started_at_ms: Some(11),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(9),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            },
            "llama_cpp_embedding",
        );

        assert_eq!(producer, Some(HostRuntimeProducer::Embedding));
    }

    #[tokio::test]
    async fn reclaim_runtime_and_reconcile_runtime_registry_stops_active_runtime_producer() {
        let controller = MockHostRuntimeController {
            mode_info: Mutex::new(HostRuntimeModeSnapshot {
                backend_name: Some("PyTorch".to_string()),
                backend_key: Some("pytorch".to_string()),
                active_model_target: Some("/models/main".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("PyTorch".to_string()),
                    runtime_instance_id: Some("pytorch-1".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            }),
            stopped_producers: Mutex::new(Vec::new()),
            stop_all_calls: Mutex::new(0),
            restore_calls: Mutex::new(Vec::new()),
            restore_should_fail: Mutex::new(false),
        };
        let registry = RuntimeRegistry::new();
        reconcile_runtime_registry_mode_info(&registry, &controller.mode_info_snapshot().await);

        let reclaim =
            reclaim_runtime_and_reconcile_runtime_registry(&controller, &registry, "pytorch")
                .await
                .expect("reclaim should succeed");

        assert_eq!(
            reclaim,
            RuntimeReclaimDisposition::stop_producer("pytorch", RuntimeRegistryStatus::Stopping)
        );
        assert_eq!(
            controller
                .stopped_producers
                .lock()
                .expect("stopped producers lock poisoned")
                .as_slice(),
            &[HostRuntimeProducer::Active]
        );
        let runtime = registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .expect("active runtime snapshot");
        assert_eq!(runtime.status, RuntimeRegistryStatus::Stopped);
    }

    #[tokio::test]
    async fn reclaim_runtime_and_reconcile_runtime_registry_syncs_before_reclaim() {
        let controller = MockHostRuntimeController {
            mode_info: Mutex::new(HostRuntimeModeSnapshot {
                backend_name: Some("PyTorch".to_string()),
                backend_key: Some("pytorch".to_string()),
                active_model_target: Some("/models/main".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("PyTorch".to_string()),
                    runtime_instance_id: Some("pytorch-2".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            }),
            stopped_producers: Mutex::new(Vec::new()),
            stop_all_calls: Mutex::new(0),
            restore_calls: Mutex::new(Vec::new()),
            restore_should_fail: Mutex::new(false),
        };
        let registry = RuntimeRegistry::new();

        let reclaim =
            reclaim_runtime_and_reconcile_runtime_registry(&controller, &registry, "pytorch")
                .await
                .expect("reclaim should succeed");

        assert_eq!(
            reclaim,
            RuntimeReclaimDisposition::stop_producer("pytorch", RuntimeRegistryStatus::Stopping)
        );
        let runtime = registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .expect("active runtime snapshot");
        assert_eq!(runtime.status, RuntimeRegistryStatus::Stopped);
    }

    #[tokio::test]
    async fn reclaim_runtime_and_reconcile_runtime_registry_stops_embedding_runtime_producer() {
        let controller = MockHostRuntimeController {
            mode_info: Mutex::new(HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: None,
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot::default()),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama_cpp_embedding".to_string()),
                    runtime_instance_id: Some("embedding-1".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            }),
            stopped_producers: Mutex::new(Vec::new()),
            stop_all_calls: Mutex::new(0),
            restore_calls: Mutex::new(Vec::new()),
            restore_should_fail: Mutex::new(false),
        };
        let registry = RuntimeRegistry::new();
        reconcile_runtime_registry_mode_info(&registry, &controller.mode_info_snapshot().await);

        let reclaim = reclaim_runtime_and_reconcile_runtime_registry(
            &controller,
            &registry,
            "llama.cpp.embedding",
        )
        .await
        .expect("reclaim should succeed");

        assert_eq!(
            reclaim,
            RuntimeReclaimDisposition::stop_producer(
                "llama.cpp.embedding",
                RuntimeRegistryStatus::Stopping,
            )
        );
        assert_eq!(
            controller
                .stopped_producers
                .lock()
                .expect("stopped producers lock poisoned")
                .as_slice(),
            &[HostRuntimeProducer::Embedding]
        );
        let runtime = registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(runtime.status, RuntimeRegistryStatus::Stopped);
    }

    #[tokio::test]
    async fn reclaim_runtime_and_reconcile_runtime_registry_keeps_other_runtime_running() {
        let controller = MockHostRuntimeController {
            mode_info: Mutex::new(HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: None,
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot::default()),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama_cpp_embedding".to_string()),
                    runtime_instance_id: Some("embedding-2".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            }),
            stopped_producers: Mutex::new(Vec::new()),
            stop_all_calls: Mutex::new(0),
            restore_calls: Mutex::new(Vec::new()),
            restore_should_fail: Mutex::new(false),
        };
        let registry = RuntimeRegistry::new();
        reconcile_runtime_registry_mode_info(&registry, &controller.mode_info_snapshot().await);
        registry.register_runtime(RuntimeRegistration::new("onnxruntime", "ONNX Runtime"));
        registry
            .transition_runtime(
                "onnxruntime",
                pantograph_runtime_registry::RuntimeTransition::Ready {
                    runtime_instance_id: Some("onnx-1".to_string()),
                },
            )
            .expect("onnx runtime should be ready");

        let reclaim =
            reclaim_runtime_and_reconcile_runtime_registry(&controller, &registry, "onnx_runtime")
                .await
                .expect("reclaim should succeed");

        assert_eq!(
            reclaim,
            RuntimeReclaimDisposition::no_action(
                "onnx-runtime",
                RuntimeRetentionReason::Status(RuntimeRegistryStatus::Stopped),
                RuntimeRegistryStatus::Stopped,
            )
        );
        assert!(controller
            .stopped_producers
            .lock()
            .expect("stopped producers lock poisoned")
            .is_empty());
    }

    #[test]
    fn reconcile_mode_info_registers_active_and_embedding_runtimes() {
        let registry = RuntimeRegistry::new();

        let snapshots = reconcile_runtime_registry_mode_info(
            &registry,
            &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-1".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp.embedding".to_string()),
                    runtime_instance_id: Some("llama-embed-1".to_string()),
                    warmup_started_at_ms: Some(11),
                    warmup_completed_at_ms: None,
                    warmup_duration_ms: None,
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            },
        );

        assert_eq!(snapshots.len(), 2);
        let active_runtime = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama_cpp")
            .expect("active runtime snapshot");
        assert_eq!(active_runtime.status, RuntimeRegistryStatus::Ready);
        assert_eq!(active_runtime.models[0].model_id, "/models/qwen.gguf");

        let embedding_runtime = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(embedding_runtime.status, RuntimeRegistryStatus::Warming);
        assert_eq!(embedding_runtime.models[0].model_id, "/models/embed.gguf");
    }

    #[test]
    fn reconcile_mode_info_stops_unobserved_runtimes_without_reservations() {
        let registry = RuntimeRegistry::new();

        reconcile_runtime_registry_mode_info(
            &registry,
            &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-1".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
        );

        let snapshots = reconcile_runtime_registry_mode_info(
            &registry,
            &HostRuntimeModeSnapshot {
                backend_name: Some("ollama".to_string()),
                backend_key: Some("ollama".to_string()),
                active_model_target: Some("llava:13b".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("ollama".to_string()),
                    runtime_instance_id: Some("ollama-1".to_string()),
                    warmup_started_at_ms: Some(30),
                    warmup_completed_at_ms: Some(35),
                    warmup_duration_ms: Some(5),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
        );

        assert_eq!(snapshots.len(), 2);
        let llama = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama_cpp")
            .expect("llama snapshot");
        assert_eq!(llama.status, RuntimeRegistryStatus::Stopped);
        assert!(llama.models.is_empty());

        let ollama = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "ollama")
            .expect("ollama snapshot");
        assert_eq!(ollama.status, RuntimeRegistryStatus::Ready);
        assert_eq!(ollama.models[0].model_id, "llava:13b");
    }

    #[test]
    fn reconcile_snapshot_override_adds_python_runtime_without_stopping_gateway_runtime() {
        let registry = RuntimeRegistry::new();

        reconcile_runtime_registry_mode_info(
            &registry,
            &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-1".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
        );

        let pytorch = reconcile_runtime_registry_snapshot_override(
            &registry,
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("PyTorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:venv_torch".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            },
            Some("/models/demo"),
        )
        .expect("python snapshot should be reconciled");

        assert_eq!(pytorch.runtime_id, "pytorch");
        assert_eq!(pytorch.display_name, "PyTorch (Python sidecar)");
        assert_eq!(pytorch.status, RuntimeRegistryStatus::Ready);
        assert_eq!(pytorch.models[0].model_id, "/models/demo");

        let snapshot = registry.snapshot();
        let llama = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("gateway runtime should remain in registry");
        assert_eq!(llama.status, RuntimeRegistryStatus::Ready);

        let pytorch = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .expect("python runtime should be present in registry");
        assert!(pytorch.backend_keys.contains(&"pytorch".to_string()));
    }

    #[test]
    fn reconcile_snapshot_override_preserves_matching_unhealthy_runtime() {
        let registry = RuntimeRegistry::new();
        registry.observe_runtime(RuntimeObservation {
            runtime_id: "pytorch".to_string(),
            display_name: "PyTorch (Python sidecar)".to_string(),
            backend_keys: vec!["pytorch".to_string()],
            model_id: Some("/models/failed.safetensors".to_string()),
            status: RuntimeRegistryStatus::Unhealthy,
            runtime_instance_id: Some("python-runtime:pytorch:venv_torch".to_string()),
            last_error: Some("probe timeout".to_string()),
        });

        let pytorch = reconcile_runtime_registry_snapshot_override(
            &registry,
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("PyTorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:venv_torch".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            },
            Some("/models/retry.safetensors"),
        )
        .expect("python snapshot should be reconciled");

        assert_eq!(pytorch.status, RuntimeRegistryStatus::Unhealthy);
        assert_eq!(pytorch.last_error.as_deref(), Some("probe timeout"));
        assert_eq!(pytorch.models[0].model_id, "/models/retry.safetensors");
    }

    #[test]
    fn reconcile_snapshot_override_marks_runtime_unhealthy_from_assessment() {
        let registry = RuntimeRegistry::new();

        let pytorch = reconcile_runtime_registry_snapshot_override_with_health_assessment(
            &registry,
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("PyTorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: false,
                last_error: Some("python sidecar crashed".to_string()),
            },
            Some("/models/retry.safetensors"),
            Some(&RuntimeHealthAssessment {
                healthy: false,
                state: RuntimeHealthState::Unhealthy {
                    reason: "python sidecar crashed".to_string(),
                },
                response_time_ms: None,
                error: Some("python sidecar crashed".to_string()),
                consecutive_failures: 1,
            }),
        )
        .expect("python snapshot should be reconciled");

        assert_eq!(pytorch.status, RuntimeRegistryStatus::Unhealthy);
        assert_eq!(
            pytorch.last_error.as_deref(),
            Some("python sidecar crashed")
        );
        assert_eq!(pytorch.models[0].model_id, "/models/retry.safetensors");
    }

    #[tokio::test]
    async fn runtime_registry_snapshot_syncs_controller_mode_info() {
        let controller = MockHostRuntimeController {
            mode_info: Mutex::new(HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-7".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp.embedding".to_string()),
                    runtime_instance_id: Some("llama-embed-7".to_string()),
                    warmup_started_at_ms: Some(11),
                    warmup_completed_at_ms: Some(16),
                    warmup_duration_ms: Some(5),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            }),
            stopped_producers: Mutex::new(Vec::new()),
            stop_all_calls: Mutex::new(0),
            restore_calls: Mutex::new(Vec::new()),
            restore_should_fail: Mutex::new(false),
        };
        let registry = RuntimeRegistry::new();

        let snapshot = runtime_registry_snapshot(&controller, &registry).await;

        assert!(snapshot
            .runtimes
            .iter()
            .any(|runtime| runtime.runtime_id == "llama_cpp"));
        assert!(snapshot
            .runtimes
            .iter()
            .any(|runtime| runtime.runtime_id == "llama.cpp.embedding"));
    }

    #[tokio::test]
    async fn stop_all_runtime_producers_and_reconcile_runtime_registry_syncs_after_stop_all() {
        let controller = MockHostRuntimeController {
            mode_info: Mutex::new(HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-9".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp.embedding".to_string()),
                    runtime_instance_id: Some("llama-embed-9".to_string()),
                    warmup_started_at_ms: Some(11),
                    warmup_completed_at_ms: Some(16),
                    warmup_duration_ms: Some(5),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            }),
            stopped_producers: Mutex::new(Vec::new()),
            stop_all_calls: Mutex::new(0),
            restore_calls: Mutex::new(Vec::new()),
            restore_should_fail: Mutex::new(false),
        };
        let registry = RuntimeRegistry::new();
        reconcile_runtime_registry_mode_info(&registry, &controller.mode_info_snapshot().await);

        stop_all_runtime_producers_and_reconcile_runtime_registry(&controller, &registry).await;

        assert_eq!(
            *controller
                .stop_all_calls
                .lock()
                .expect("stop-all calls lock poisoned"),
            1
        );
        let snapshot = registry.snapshot();
        assert!(snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.status == RuntimeRegistryStatus::Stopped));
    }

    #[tokio::test]
    async fn restore_runtime_and_reconcile_runtime_registry_syncs_after_restore() {
        let controller = MockHostRuntimeController {
            mode_info: Mutex::new(HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot::default()),
                embedding_runtime: None,
            }),
            stopped_producers: Mutex::new(Vec::new()),
            stop_all_calls: Mutex::new(0),
            restore_calls: Mutex::new(Vec::new()),
            restore_should_fail: Mutex::new(false),
        };
        let registry = RuntimeRegistry::new();

        restore_runtime_and_reconcile_runtime_registry(
            &controller,
            &registry,
            Some(inference::BackendConfig::default()),
        )
        .await
        .expect("restore should succeed");

        assert_eq!(
            controller
                .restore_calls
                .lock()
                .expect("restore calls lock poisoned")
                .len(),
            1
        );
        let runtime = registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("restored runtime snapshot");
        assert_eq!(runtime.status, RuntimeRegistryStatus::Ready);
        assert_eq!(
            runtime.runtime_instance_id.as_deref(),
            Some("llama-main-restored")
        );
    }

    struct HealthAwareHostRuntimeController {
        mode_info: HostRuntimeModeSnapshot,
        health_assessments: RuntimeHealthAssessmentSnapshot,
    }

    #[async_trait]
    impl HostRuntimeRegistryController for HealthAwareHostRuntimeController {
        async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
            self.mode_info.clone()
        }

        async fn stop_runtime_producer(&self, _producer: HostRuntimeProducer) {}

        async fn runtime_health_assessment_snapshot(&self) -> RuntimeHealthAssessmentSnapshot {
            self.health_assessments.clone()
        }
    }

    #[tokio::test]
    async fn sync_runtime_registry_uses_controller_health_assessment_snapshot() {
        let controller = HealthAwareHostRuntimeController {
            mode_info: HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-10".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp.embedding".to_string()),
                    runtime_instance_id: Some("llama-embed-10".to_string()),
                    warmup_started_at_ms: Some(11),
                    warmup_completed_at_ms: Some(16),
                    warmup_duration_ms: Some(5),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            },
            health_assessments: RuntimeHealthAssessmentSnapshot {
                active: None,
                embedding: Some(crate::runtime_health::RuntimeHealthAssessmentRecord {
                    runtime_id: "llama.cpp.embedding".to_string(),
                    runtime_instance_id: Some("llama-embed-10".to_string()),
                    assessment: RuntimeHealthAssessment {
                        healthy: false,
                        state: RuntimeHealthState::Unhealthy {
                            reason: "Connection refused".to_string(),
                        },
                        response_time_ms: None,
                        error: Some("Connection refused".to_string()),
                        consecutive_failures: 3,
                    },
                }),
            },
        };
        let registry = RuntimeRegistry::new();

        sync_runtime_registry(&controller, &registry).await;

        let runtime = registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(runtime.status, RuntimeRegistryStatus::Unhealthy);
        assert_eq!(runtime.last_error.as_deref(), Some("Connection refused"));
    }

    #[test]
    fn reconcile_mode_info_marks_active_runtime_unhealthy_from_health_assessment() {
        let registry = RuntimeRegistry::new();

        let snapshots = reconcile_runtime_registry_mode_info_with_active_health_assessment(
            &registry,
            &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-unhealthy".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
            Some(&RuntimeHealthAssessment {
                healthy: false,
                state: RuntimeHealthState::Unhealthy {
                    reason: "Request timeout".to_string(),
                },
                response_time_ms: None,
                error: Some("Request timeout".to_string()),
                consecutive_failures: 3,
            }),
        );

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].runtime_id, "llama_cpp");
        assert_eq!(snapshots[0].status, RuntimeRegistryStatus::Unhealthy);
        assert_eq!(snapshots[0].last_error.as_deref(), Some("Request timeout"));
        assert_eq!(
            snapshots[0].runtime_instance_id.as_deref(),
            Some("llama-main-unhealthy")
        );
    }

    #[test]
    fn reconcile_mode_info_keeps_ready_runtime_when_health_is_only_degraded() {
        let registry = RuntimeRegistry::new();

        let snapshots = reconcile_runtime_registry_mode_info_with_active_health_assessment(
            &registry,
            &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-ready".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: None,
            },
            Some(&RuntimeHealthAssessment {
                healthy: true,
                state: RuntimeHealthState::Degraded {
                    reason: "HTTP 503".to_string(),
                },
                response_time_ms: Some(42),
                error: Some("HTTP 503".to_string()),
                consecutive_failures: 1,
            }),
        );

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].runtime_id, "llama_cpp");
        assert_eq!(snapshots[0].status, RuntimeRegistryStatus::Ready);
        assert_eq!(snapshots[0].last_error, None);
    }

    #[test]
    fn reconcile_mode_info_marks_embedding_runtime_unhealthy_from_health_assessment() {
        let registry = RuntimeRegistry::new();

        let snapshots = reconcile_runtime_registry_mode_info_with_health_assessments(
            &registry,
            &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/qwen.gguf".to_string()),
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp".to_string()),
                    runtime_instance_id: Some("llama-main-ready".to_string()),
                    warmup_started_at_ms: Some(10),
                    warmup_completed_at_ms: Some(20),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
                embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                    runtime_id: Some("llama.cpp.embedding".to_string()),
                    runtime_instance_id: Some("llama-embed-unhealthy".to_string()),
                    warmup_started_at_ms: Some(11),
                    warmup_completed_at_ms: Some(16),
                    warmup_duration_ms: Some(5),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                }),
            },
            None,
            Some(&RuntimeHealthAssessment {
                healthy: false,
                state: RuntimeHealthState::Unhealthy {
                    reason: "Connection refused".to_string(),
                },
                response_time_ms: None,
                error: Some("Connection refused".to_string()),
                consecutive_failures: 3,
            }),
        );

        let embedding = snapshots
            .iter()
            .find(|snapshot| snapshot.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(embedding.status, RuntimeRegistryStatus::Unhealthy);
        assert_eq!(embedding.last_error.as_deref(), Some("Connection refused"));
        assert_eq!(
            embedding.runtime_instance_id.as_deref(),
            Some("llama-embed-unhealthy")
        );
    }
}
