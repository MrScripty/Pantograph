//! Runtime-registry inspection and targeted reclaim commands.

use serde::{Deserialize, Serialize};
use tauri::{command, State};

use crate::llm::runtime_registry::{
    reclaim_runtime_and_sync_runtime_registry, sync_runtime_registry_from_gateway,
};
use crate::llm::{SharedGateway, SharedRuntimeRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRegistryReclaimResponse {
    pub reclaim: pantograph_runtime_registry::RuntimeReclaimDisposition,
    pub snapshot: pantograph_runtime_registry::RuntimeRegistrySnapshot,
}

async fn runtime_registry_snapshot(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
) -> pantograph_runtime_registry::RuntimeRegistrySnapshot {
    sync_runtime_registry_from_gateway(gateway, runtime_registry).await;
    runtime_registry.snapshot()
}

async fn reclaim_runtime(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    runtime_id: &str,
) -> Result<RuntimeRegistryReclaimResponse, String> {
    sync_runtime_registry_from_gateway(gateway, runtime_registry).await;
    let reclaim = reclaim_runtime_and_sync_runtime_registry(gateway, runtime_registry, runtime_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(RuntimeRegistryReclaimResponse {
        reclaim,
        snapshot: runtime_registry.snapshot(),
    })
}

#[command]
pub async fn get_runtime_registry_snapshot(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
) -> Result<pantograph_runtime_registry::RuntimeRegistrySnapshot, String> {
    Ok(runtime_registry_snapshot(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn reclaim_runtime_registry_runtime(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    runtime_id: String,
) -> Result<RuntimeRegistryReclaimResponse, String> {
    reclaim_runtime(gateway.inner(), runtime_registry.inner(), &runtime_id).await
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use async_trait::async_trait;
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::{EmbeddingMemoryMode, LlamaCppEmbeddingRuntime};
    use tokio::sync::mpsc;

    use super::{reclaim_runtime, runtime_registry_snapshot};
    use crate::llm::{InferenceGateway, RuntimeRegistry, SharedGateway, SharedRuntimeRegistry};

    struct MockProcessHandle;

    impl ProcessHandle for MockProcessHandle {
        fn pid(&self) -> u32 {
            21
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
            Err("spawn not used in runtime registry command tests".to_string())
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }
    }

    #[tokio::test]
    async fn runtime_registry_snapshot_syncs_embedding_runtime_observation() {
        let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));
        gateway.init().await;

        let mut server = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-10".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        });
        gateway.set_test_embedding_server(server).await;

        let runtime_registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let snapshot = runtime_registry_snapshot(&gateway, &runtime_registry).await;

        let runtime = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(
            runtime.runtime_instance_id.as_deref(),
            Some("llama-cpp-embedding-10")
        );
        assert_eq!(
            runtime.status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Ready
        );
    }

    #[tokio::test]
    async fn reclaim_runtime_returns_updated_registry_snapshot() {
        let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));
        gateway.init().await;

        let mut server = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-11".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        });
        gateway.set_test_embedding_server(server).await;

        let runtime_registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let response = reclaim_runtime(&gateway, &runtime_registry, "llama_cpp_embedding")
            .await
            .expect("reclaim should succeed");

        assert_eq!(
            response.reclaim,
            pantograph_runtime_registry::RuntimeReclaimDisposition::stop_producer(
                "llama.cpp.embedding",
                pantograph_runtime_registry::RuntimeRegistryStatus::Stopping,
            )
        );
        let runtime = response
            .snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(
            runtime.status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
        );
        assert!(runtime.runtime_instance_id.is_none());
    }
}
