//! Tauri-side re-export of backend-owned runtime-registry helpers.

pub use pantograph_embedded_runtime::runtime_registry::{
    reconcile_runtime_registry_mode_info, reconcile_runtime_registry_snapshot_override,
};
use pantograph_embedded_runtime::HostRuntimeModeSnapshot;
pub use pantograph_runtime_registry::{RuntimeRegistry, SharedRuntimeRegistry};

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use async_trait::async_trait;
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::EmbeddingMemoryMode;
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
}
