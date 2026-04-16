//! Runtime-registry inspection and targeted reclaim commands.

use crate::config::ServerModeInfo;
use crate::llm::commands::shared::synced_server_mode_info;
use crate::llm::health_monitor::{HealthCheckResult, SharedHealthMonitor};
use crate::llm::recovery::{RecoveryConfig, SharedRecoveryManager};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Manager, State};

use crate::llm::runtime_registry::{
    reclaim_runtime_and_sync_runtime_registry,
    runtime_registry_snapshot as synced_runtime_registry_snapshot,
};
use crate::llm::{SharedGateway, SharedRuntimeRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRegistryReclaimResponse {
    pub reclaim: pantograph_runtime_registry::RuntimeReclaimDisposition,
    pub snapshot: pantograph_runtime_registry::RuntimeRegistrySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRecoveryDebugState {
    pub in_progress: bool,
    pub attempt_count: u32,
    pub config: RecoveryConfig,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeDebugSnapshot {
    pub mode_info: ServerModeInfo,
    pub snapshot: pantograph_runtime_registry::RuntimeRegistrySnapshot,
    pub health_monitor_running: bool,
    pub last_health_check: Option<HealthCheckResult>,
    pub recovery: Option<RuntimeRecoveryDebugState>,
}

async fn runtime_registry_snapshot_response(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
) -> pantograph_runtime_registry::RuntimeRegistrySnapshot {
    synced_runtime_registry_snapshot(gateway.as_ref(), runtime_registry).await
}

async fn runtime_debug_snapshot_response(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    health_monitor: Option<&SharedHealthMonitor>,
    recovery_manager: Option<&SharedRecoveryManager>,
) -> RuntimeDebugSnapshot {
    let mode_info = synced_server_mode_info(gateway, runtime_registry).await;
    let health_monitor_running = health_monitor
        .map(|monitor| monitor.is_running())
        .unwrap_or(false);
    let last_health_check = match health_monitor {
        Some(monitor) => monitor.last_result().await,
        None => None,
    };
    let recovery = match recovery_manager {
        Some(manager) => Some(RuntimeRecoveryDebugState {
            in_progress: manager.is_recovering(),
            attempt_count: manager.attempt_count(),
            config: manager.config().clone(),
            last_error: manager.last_error().await,
        }),
        None => None,
    };

    RuntimeDebugSnapshot {
        mode_info,
        snapshot: runtime_registry.snapshot(),
        health_monitor_running,
        last_health_check,
        recovery,
    }
}

async fn reclaim_runtime(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    runtime_id: &str,
) -> Result<RuntimeRegistryReclaimResponse, String> {
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
    Ok(runtime_registry_snapshot_response(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn get_runtime_debug_snapshot(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
) -> Result<RuntimeDebugSnapshot, String> {
    let health_monitor = app
        .try_state::<SharedHealthMonitor>()
        .map(|monitor| (*monitor).clone());
    let recovery_manager = app
        .try_state::<SharedRecoveryManager>()
        .map(|manager| (*manager).clone());

    Ok(runtime_debug_snapshot_response(
        gateway.inner(),
        runtime_registry.inner(),
        health_monitor.as_ref(),
        recovery_manager.as_ref(),
    )
    .await)
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

    use super::{
        reclaim_runtime, runtime_debug_snapshot_response, runtime_registry_snapshot_response,
    };
    use crate::llm::health_monitor::{
        HealthCheckResult, HealthMonitor, HealthMonitorConfig, HealthStatus, SharedHealthMonitor,
    };
    use crate::llm::recovery::{RecoveryManager, SharedRecoveryManager};
    use crate::llm::{InferenceGateway, RuntimeRegistry, SharedGateway, SharedRuntimeRegistry};
    use chrono::Utc;

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
        let snapshot = runtime_registry_snapshot_response(&gateway, &runtime_registry).await;

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

    #[tokio::test]
    async fn runtime_debug_snapshot_includes_synced_runtime_and_recovery_state() {
        let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));
        gateway.init().await;

        let mut server = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-12".to_string()),
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
        let health_monitor: SharedHealthMonitor =
            Arc::new(HealthMonitor::new(HealthMonitorConfig::default()));
        health_monitor
            .set_test_last_result(Some(HealthCheckResult {
                healthy: true,
                status: HealthStatus::Healthy,
                response_time_ms: Some(25),
                error: None,
                timestamp: Utc::now(),
                consecutive_failures: 0,
            }))
            .await;
        let recovery_manager: SharedRecoveryManager = Arc::new(RecoveryManager::default());

        let response = runtime_debug_snapshot_response(
            &gateway,
            &runtime_registry,
            Some(&health_monitor),
            Some(&recovery_manager),
        )
        .await;

        let runtime = response
            .snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime snapshot");
        assert_eq!(
            runtime.runtime_instance_id.as_deref(),
            Some("llama-cpp-embedding-12")
        );
        assert_eq!(
            response.mode_info.embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert!(!response.health_monitor_running);
        assert_eq!(
            response
                .last_health_check
                .as_ref()
                .and_then(|result| result.response_time_ms),
            Some(25)
        );
        assert_eq!(
            response
                .recovery
                .as_ref()
                .map(|recovery| recovery.attempt_count),
            Some(0)
        );
        assert_eq!(
            response
                .recovery
                .as_ref()
                .and_then(|recovery| recovery.last_error.as_deref()),
            None
        );
    }
}
