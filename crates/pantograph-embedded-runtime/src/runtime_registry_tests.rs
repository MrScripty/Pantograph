use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;

use super::*;
use crate::runtime_health::{
    RuntimeHealthAssessment, RuntimeHealthAssessmentSnapshot, RuntimeHealthState,
};
use pantograph_runtime_registry::{
    RuntimeReclaimDisposition, RuntimeRegistration, RuntimeRegistryStatus, RuntimeRetentionHint,
    RuntimeRetentionReason,
};

#[path = "runtime_registry_tests/health_warmup.rs"]
mod health_warmup;
#[path = "runtime_registry_tests/lifecycle.rs"]
mod lifecycle;
#[path = "runtime_registry_tests/observations.rs"]
mod observations;

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
                mode_info.embedding_runtime = Some(inference::RuntimeLifecycleSnapshot::default());
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

struct HealthAwareHostRuntimeController {
    mode_info: HostRuntimeModeSnapshot,
    health_assessments: RuntimeHealthAssessmentSnapshot,
}

struct HealthAwareLifecycleController {
    mode_info: HostRuntimeModeSnapshot,
    health_assessments: RuntimeHealthAssessmentSnapshot,
    restore_calls: Mutex<Vec<Option<inference::BackendConfig>>>,
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

#[async_trait]
impl HostRuntimeRegistryController for HealthAwareLifecycleController {
    async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
        self.mode_info.clone()
    }

    async fn stop_runtime_producer(&self, _producer: HostRuntimeProducer) {}

    async fn runtime_health_assessment_snapshot(&self) -> RuntimeHealthAssessmentSnapshot {
        self.health_assessments.clone()
    }
}

#[async_trait]
impl HostRuntimeRegistryLifecycleController for HealthAwareLifecycleController {
    async fn stop_all_runtime_producers(&self) {}

    async fn restore_runtime(
        &self,
        restore_config: Option<inference::BackendConfig>,
    ) -> Result<(), inference::GatewayError> {
        self.restore_calls
            .lock()
            .expect("restore calls lock poisoned")
            .push(restore_config);
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

    let reclaim = reclaim_runtime_and_reconcile_runtime_registry(&controller, &registry, "pytorch")
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

    let reclaim = reclaim_runtime_and_reconcile_runtime_registry(&controller, &registry, "pytorch")
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
    assert!(
        controller
            .stopped_producers
            .lock()
            .expect("stopped producers lock poisoned")
            .is_empty()
    );
}

#[tokio::test]
async fn release_reservation_and_reconcile_runtime_registry_reclaims_evicted_runtime() {
    let controller = MockHostRuntimeController {
        mode_info: Mutex::new(HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-release".to_string()),
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
    let lease = registry
        .acquire_reservation(active_runtime_reservation_request(
            &registry,
            &controller.mode_info_snapshot().await,
            "wf-1",
            Some("session-release"),
            Some("interactive"),
            None,
            RuntimeRetentionHint::Ephemeral,
        ))
        .expect("reservation should be created");

    let disposition = release_reservation_and_reconcile_runtime_registry(
        &controller,
        &registry,
        lease.reservation_id,
    )
    .await
    .expect("release should succeed")
    .expect("disposition should be returned");

    assert_eq!(
        disposition.decision,
        pantograph_runtime_registry::RuntimeRetentionDecision::Evict
    );
    assert_eq!(disposition.runtime_id, "llama_cpp");
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
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("runtime should remain registered");
    assert_eq!(runtime.status, RuntimeRegistryStatus::Stopped);
    assert!(runtime.runtime_instance_id.is_none());
}
