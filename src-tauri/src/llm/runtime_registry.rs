//! Tauri-side re-export of backend-owned runtime-registry helpers.

pub use pantograph_embedded_runtime::runtime_registry::{
    reconcile_runtime_registry_mode_info, reconcile_runtime_registry_snapshot_override,
};
use pantograph_embedded_runtime::HostRuntimeModeSnapshot;
use pantograph_runtime_identity::canonical_runtime_id;
pub use pantograph_runtime_registry::{
    RuntimeReclaimAction, RuntimeReclaimDisposition, RuntimeRegistry, RuntimeRegistryError,
    SharedRuntimeRegistry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostRuntimeProducer {
    Active,
    Embedding,
}

fn active_runtime_id(mode_info: &inference::ServerModeInfo) -> Option<String> {
    mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_id.as_deref())
        .or(mode_info.backend_key.as_deref())
        .or(mode_info.backend_name.as_deref())
        .map(canonical_runtime_id)
}

fn embedding_runtime_id(mode_info: &inference::ServerModeInfo) -> Option<String> {
    mode_info.embedding_runtime.as_ref().map(|snapshot| {
        snapshot
            .runtime_id
            .as_deref()
            .map(canonical_runtime_id)
            .unwrap_or_else(|| "llama.cpp.embedding".to_string())
    })
}

fn live_host_runtime_producer(
    mode_info: &inference::ServerModeInfo,
    runtime_id: &str,
) -> Option<HostRuntimeProducer> {
    let runtime_id = canonical_runtime_id(runtime_id);

    if mode_info.ready
        && mode_info
            .active_runtime
            .as_ref()
            .map(|snapshot| snapshot.active)
            .unwrap_or(false)
        && active_runtime_id(mode_info).as_deref() == Some(runtime_id.as_str())
    {
        return Some(HostRuntimeProducer::Active);
    }

    if mode_info
        .embedding_runtime
        .as_ref()
        .map(|snapshot| snapshot.active)
        .unwrap_or(false)
        && embedding_runtime_id(mode_info).as_deref() == Some(runtime_id.as_str())
    {
        return Some(HostRuntimeProducer::Embedding);
    }

    None
}

pub async fn sync_runtime_registry_from_gateway(
    gateway: &crate::llm::gateway::InferenceGateway,
    registry: &RuntimeRegistry,
) {
    let mode_info = HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
    reconcile_runtime_registry_mode_info(registry, &mode_info);
}

pub async fn stop_all_and_sync_runtime_registry(
    gateway: &crate::llm::gateway::InferenceGateway,
    registry: &RuntimeRegistry,
) {
    gateway.stop_all().await;
    sync_runtime_registry_from_gateway(gateway, registry).await;
}

pub async fn restore_runtime_and_sync_runtime_registry(
    gateway: &crate::llm::gateway::InferenceGateway,
    registry: &RuntimeRegistry,
    restore_config: Option<inference::BackendConfig>,
) -> Result<(), inference::GatewayError> {
    let result = gateway.restore_inference_runtime(restore_config).await;
    sync_runtime_registry_from_gateway(gateway, registry).await;
    result
}

pub async fn reclaim_runtime_and_sync_runtime_registry(
    gateway: &crate::llm::gateway::InferenceGateway,
    registry: &RuntimeRegistry,
    runtime_id: &str,
) -> Result<RuntimeReclaimDisposition, RuntimeRegistryError> {
    let mode_info = gateway.mode_info().await;
    let live_producer = live_host_runtime_producer(&mode_info, runtime_id);
    let reclaim = registry.reclaim_runtime(runtime_id, live_producer.is_some())?;

    if reclaim.action == RuntimeReclaimAction::StopProducer {
        match live_producer {
            Some(HostRuntimeProducer::Active) => gateway.stop().await,
            Some(HostRuntimeProducer::Embedding) => gateway.stop_embedding_server().await,
            None => {}
        }
    }

    sync_runtime_registry_from_gateway(gateway, registry).await;
    Ok(reclaim)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use futures_util::stream;
    use futures_util::Stream;
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        InferenceBackend,
    };
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::EmbeddingMemoryMode;
    use inference::{ImageGenerationRequest, ImageGenerationResult, RerankRequest, RerankResponse};
    use pantograph_runtime_registry::{
        RuntimeRegistration, RuntimeRetentionReason, RuntimeTransition,
    };
    use tokio::sync::mpsc;

    use super::*;

    struct MockProcessHandle;

    impl ProcessHandle for MockProcessHandle {
        fn pid(&self) -> u32 {
            17
        }

        fn kill(&self) -> Result<(), String> {
            Ok(())
        }
    }

    struct MockProcessSpawner;

    #[async_trait]
    impl ProcessSpawner for MockProcessSpawner {
        async fn spawn_sidecar(
            &self,
            _sidecar_name: &str,
            _args: &[&str],
        ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
            Err("spawn should not be called in runtime registry tests".to_string())
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }
    }

    struct MockInferenceBackend {
        ready: Arc<Mutex<bool>>,
    }

    impl MockInferenceBackend {
        fn new() -> Self {
            Self {
                ready: Arc::new(Mutex::new(false)),
            }
        }
    }

    #[async_trait]
    impl InferenceBackend for MockInferenceBackend {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn description(&self) -> &'static str {
            "Mock backend for runtime-registry tests"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            *self.ready.lock().expect("mock backend ready lock poisoned") = true;
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            })
        }

        fn stop(&mut self) {
            *self.ready.lock().expect("mock backend ready lock poisoned") = false;
        }

        fn is_ready(&self) -> bool {
            *self.ready.lock().expect("mock backend ready lock poisoned")
        }

        async fn health_check(&self) -> bool {
            self.is_ready()
        }

        fn base_url(&self) -> Option<String> {
            Some("http://127.0.0.1:11434".to_string())
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
        {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> Result<Vec<inference::EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
            Err(BackendError::Inference(
                "rerank should not be called in runtime registry tests".to_string(),
            ))
        }

        async fn generate_image(
            &self,
            _request: ImageGenerationRequest,
        ) -> Result<ImageGenerationResult, BackendError> {
            Err(BackendError::Inference(
                "image generation should not be called in runtime registry tests".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn sync_runtime_registry_from_gateway_preserves_embedding_runtime_observation() {
        let gateway = crate::llm::gateway::InferenceGateway::new(Arc::new(MockProcessSpawner));
        gateway.init().await;

        let mut server = inference::LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-5".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        });
        gateway.set_test_embedding_server(server).await;

        let registry = RuntimeRegistry::new();
        sync_runtime_registry_from_gateway(&gateway, &registry).await;

        let snapshot = registry.snapshot();
        assert!(snapshot
            .runtimes
            .iter()
            .any(|runtime| runtime.runtime_id == "llama.cpp.embedding"));
    }

    #[tokio::test]
    async fn stop_all_and_sync_runtime_registry_stops_embedding_runtime_observation() {
        let gateway = crate::llm::gateway::InferenceGateway::new(Arc::new(MockProcessSpawner));
        gateway.init().await;

        let mut server = inference::LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-6".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        });
        gateway.set_test_embedding_server(server).await;

        let registry = RuntimeRegistry::new();
        sync_runtime_registry_from_gateway(&gateway, &registry).await;

        stop_all_and_sync_runtime_registry(&gateway, &registry).await;

        let snapshot = registry.snapshot();
        let embedding_runtime = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(
            embedding_runtime.status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
        );
        assert!(embedding_runtime.models.is_empty());
        assert!(embedding_runtime.runtime_instance_id.is_none());
    }

    #[test]
    fn live_host_runtime_producer_matches_active_and_embedding_runtime_aliases() {
        let mode_info = inference::ServerModeInfo {
            backend_name: Some("PyTorch".to_string()),
            backend_key: Some("pytorch".to_string()),
            mode: "sidecar_inference".to_string(),
            ready: true,
            url: Some("http://127.0.0.1:11434".to_string()),
            model_path: None,
            is_embedding_mode: false,
            active_model_target: Some("/models/main".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("PyTorch".to_string()),
                runtime_instance_id: Some("pytorch-1".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama_cpp_embedding".to_string()),
                runtime_instance_id: Some("embedding-1".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        };

        assert_eq!(
            live_host_runtime_producer(&mode_info, "pytorch"),
            Some(HostRuntimeProducer::Active)
        );
        assert_eq!(
            live_host_runtime_producer(&mode_info, "llama.cpp.embedding"),
            Some(HostRuntimeProducer::Embedding)
        );
        assert_eq!(live_host_runtime_producer(&mode_info, "onnx-runtime"), None);
    }

    #[tokio::test]
    async fn reclaim_runtime_and_sync_runtime_registry_stops_active_runtime_producer() {
        let gateway = crate::llm::gateway::InferenceGateway::with_test_backend(
            Box::new(MockInferenceBackend::new()),
            "PyTorch",
            Arc::new(MockProcessSpawner),
        );
        gateway.init().await;
        gateway
            .start(&BackendConfig::default())
            .await
            .expect("gateway should start");

        let registry = RuntimeRegistry::new();
        sync_runtime_registry_from_gateway(&gateway, &registry).await;

        let reclaim = reclaim_runtime_and_sync_runtime_registry(&gateway, &registry, "pytorch")
            .await
            .expect("reclaim should succeed");

        assert_eq!(
            reclaim,
            RuntimeReclaimDisposition::stop_producer(
                "pytorch",
                pantograph_runtime_registry::RuntimeRegistryStatus::Stopping,
            )
        );

        let snapshot = registry.snapshot();
        let runtime = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .expect("active runtime snapshot");
        assert_eq!(
            runtime.status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
        );
        assert!(runtime.runtime_instance_id.is_none());
        assert!(runtime.models.is_empty());
    }

    #[tokio::test]
    async fn reclaim_runtime_and_sync_runtime_registry_stops_embedding_runtime_producer() {
        let gateway = crate::llm::gateway::InferenceGateway::new(Arc::new(MockProcessSpawner));
        gateway.init().await;

        let mut server = inference::LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-7".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        });
        gateway.set_test_embedding_server(server).await;

        let registry = RuntimeRegistry::new();
        sync_runtime_registry_from_gateway(&gateway, &registry).await;

        let reclaim =
            reclaim_runtime_and_sync_runtime_registry(&gateway, &registry, "llama_cpp_embedding")
                .await
                .expect("reclaim should succeed");

        assert_eq!(
            reclaim,
            RuntimeReclaimDisposition::stop_producer(
                "llama.cpp.embedding",
                pantograph_runtime_registry::RuntimeRegistryStatus::Stopping,
            )
        );

        let snapshot = registry.snapshot();
        let runtime = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(
            runtime.status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
        );
        assert!(runtime.runtime_instance_id.is_none());
        assert!(runtime.models.is_empty());
    }

    #[tokio::test]
    async fn reclaim_runtime_and_sync_runtime_registry_keeps_other_live_producers_running() {
        let gateway = crate::llm::gateway::InferenceGateway::new(Arc::new(MockProcessSpawner));
        gateway.init().await;

        let mut server = inference::LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-8".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        });
        gateway.set_test_embedding_server(server).await;

        let registry = RuntimeRegistry::new();
        sync_runtime_registry_from_gateway(&gateway, &registry).await;
        registry.register_runtime(RuntimeRegistration::new("onnxruntime", "ONNX Runtime"));
        registry
            .transition_runtime(
                "onnxruntime",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("onnx-runtime-1".to_string()),
                },
            )
            .expect("onnx runtime should be ready");

        let reclaim =
            reclaim_runtime_and_sync_runtime_registry(&gateway, &registry, "onnx_runtime")
                .await
                .expect("reclaim should succeed");

        assert_eq!(
            reclaim,
            RuntimeReclaimDisposition::no_action(
                "onnx-runtime",
                RuntimeRetentionReason::Evictable,
                pantograph_runtime_registry::RuntimeRegistryStatus::Stopped,
            )
        );

        let snapshot = registry.snapshot();
        let embedding_runtime = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(
            embedding_runtime.status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Ready
        );
        assert_eq!(
            embedding_runtime.runtime_instance_id.as_deref(),
            Some("llama-cpp-embedding-8")
        );

        let onnx_runtime = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "onnx-runtime")
            .expect("onnx runtime snapshot");
        assert_eq!(
            onnx_runtime.status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
        );
        assert!(onnx_runtime.runtime_instance_id.is_none());
    }
}
