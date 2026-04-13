//! Embedding server and memory mode commands.

use super::shared::SharedAppConfig;
use crate::config::EmbeddingMemoryMode;
use crate::llm::gateway::SharedGateway;
use tauri::{AppHandle, Manager, State, command};

async fn embedding_runtime_lifecycle_snapshot(
    gateway: &SharedGateway,
) -> Option<inference::RuntimeLifecycleSnapshot> {
    gateway.embedding_runtime_lifecycle_snapshot().await
}

/// Get the current embedding memory mode
#[command]
pub async fn get_embedding_memory_mode(
    config: State<'_, SharedAppConfig>,
) -> Result<String, String> {
    let config_guard = config.read().await;
    let mode = match config_guard.embedding_memory_mode {
        EmbeddingMemoryMode::CpuParallel => "cpu_parallel",
        EmbeddingMemoryMode::GpuParallel => "gpu_parallel",
        EmbeddingMemoryMode::Sequential => "sequential",
    };
    Ok(mode.to_string())
}

/// Set the embedding memory mode
/// Note: This saves the config but doesn't restart the embedding server.
/// Call start_sidecar_inference to apply the new mode.
#[command]
pub async fn set_embedding_memory_mode(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    mode: String,
) -> Result<(), String> {
    let new_mode = match mode.as_str() {
        "cpu_parallel" => EmbeddingMemoryMode::CpuParallel,
        "gpu_parallel" => EmbeddingMemoryMode::GpuParallel,
        "sequential" => EmbeddingMemoryMode::Sequential,
        _ => return Err(format!("Invalid embedding memory mode: {}", mode)),
    };

    {
        let mut config_guard = config.write().await;
        config_guard.embedding_memory_mode = new_mode;
    }

    // Save config to disk
    let config_guard = config.read().await;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    config_guard
        .save(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Set embedding memory mode to: {}", mode);
    Ok(())
}

/// Check if the embedding server is ready
#[command]
pub async fn is_embedding_server_ready(gateway: State<'_, SharedGateway>) -> Result<bool, String> {
    Ok(gateway.is_embedding_server_ready().await)
}

/// Get the embedding server URL if available
#[command]
pub async fn get_embedding_server_url(
    gateway: State<'_, SharedGateway>,
) -> Result<Option<String>, String> {
    Ok(gateway.embedding_url().await)
}

/// Get the backend-owned lifecycle snapshot for the dedicated embedding server.
#[command]
pub async fn get_embedding_runtime_lifecycle_snapshot(
    gateway: State<'_, SharedGateway>,
) -> Result<Option<inference::RuntimeLifecycleSnapshot>, String> {
    Ok(embedding_runtime_lifecycle_snapshot(gateway.inner()).await)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use async_trait::async_trait;
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use tokio::sync::mpsc;

    use super::embedding_runtime_lifecycle_snapshot;
    use crate::config::EmbeddingMemoryMode;
    use crate::llm::{InferenceGateway, SharedGateway, embedding_server::EmbeddingServer};

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
            Err("spawn not used in embedding command tests".to_string())
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }
    }

    #[tokio::test]
    async fn embedding_runtime_lifecycle_snapshot_reads_backend_owned_snapshot() {
        let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));
        let mut server = EmbeddingServer::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-7".to_string()),
            warmup_started_at_ms: Some(100),
            warmup_completed_at_ms: Some(110),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("reused_embedding_runtime".to_string()),
            active: true,
            last_error: None,
        });
        gateway.set_test_embedding_server(server).await;

        let snapshot = embedding_runtime_lifecycle_snapshot(&gateway)
            .await
            .expect("snapshot should exist");

        assert_eq!(snapshot.runtime_id.as_deref(), Some("llama.cpp.embedding"));
        assert_eq!(
            snapshot.runtime_instance_id.as_deref(),
            Some("llama-cpp-embedding-7")
        );
        assert_eq!(snapshot.runtime_reused, Some(true));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("reused_embedding_runtime")
        );
    }

    #[tokio::test]
    async fn embedding_runtime_lifecycle_snapshot_returns_none_without_server() {
        let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));

        assert_eq!(embedding_runtime_lifecycle_snapshot(&gateway).await, None);
    }
}
