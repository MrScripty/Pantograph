//! LLM server lifecycle management commands.

use super::config::list_devices;
use super::shared::{synced_server_mode_info, SharedAppConfig};
use crate::agent::rag::SharedRagManager;
use crate::config::{EmbeddingMemoryMode, ServerModeInfo};
use crate::llm::startup::{
    build_configured_embedding_request, build_configured_inference_request,
    build_explicit_llamacpp_inference_request, build_external_inference_request,
};
use crate::llm::{sync_rag_embedding_url_from_gateway, SharedGateway, SharedRuntimeRegistry};
use pantograph_embedded_runtime::embedding_workflow::resolve_embedding_model_path;
use tauri::{command, AppHandle, State};

#[command]
pub async fn connect_to_server(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    rag_manager: State<'_, SharedRagManager>,
    url: String,
) -> Result<ServerModeInfo, String> {
    let external_backend_key = gateway
        .available_backends()
        .into_iter()
        .find(|backend| backend.capabilities.external_connection)
        .map(|backend| backend.backend_key)
        .ok_or_else(|| "No backend supports external server attachment".to_string())?;

    if gateway.mode_info().await.backend_key.as_deref() != Some(external_backend_key.as_str()) {
        gateway
            .switch_backend(&external_backend_key)
            .await
            .map_err(|e| e.to_string())?;
    }

    let backend_config = gateway
        .build_inference_start_config(build_external_inference_request(&url)?)
        .await
        .map_err(|e| e.to_string())?;

    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;
    sync_rag_embedding_url_from_gateway(gateway.inner(), rag_manager.inner()).await;

    Ok(synced_server_mode_info(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn start_sidecar_llm(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    rag_manager: State<'_, SharedRagManager>,
    config: State<'_, SharedAppConfig>,
    model_path: String,
    mmproj_path: String,
) -> Result<ServerModeInfo, String> {
    if gateway.mode_info().await.backend_key.as_deref() != Some("llama_cpp") {
        gateway
            .switch_backend("llama_cpp")
            .await
            .map_err(|e| e.to_string())?;
    }

    let config_guard = config.read().await;
    let inference_request =
        build_explicit_llamacpp_inference_request(&model_path, &mmproj_path, &config_guard.device);
    drop(config_guard);

    let backend_config = gateway
        .build_inference_start_config(inference_request)
        .await
        .map_err(|e| e.to_string())?;

    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;
    sync_rag_embedding_url_from_gateway(gateway.inner(), rag_manager.inner()).await;

    Ok(synced_server_mode_info(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn get_llm_status(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
) -> Result<ServerModeInfo, String> {
    Ok(synced_server_mode_info(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn stop_llm(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    rag_manager: State<'_, SharedRagManager>,
) -> Result<ServerModeInfo, String> {
    gateway.stop().await;
    sync_rag_embedding_url_from_gateway(gateway.inner(), rag_manager.inner()).await;
    Ok(synced_server_mode_info(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn start_sidecar_inference(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
) -> Result<ServerModeInfo, String> {
    let config_guard = config.read().await;
    let backend_name = gateway.current_backend_name().await;
    log::info!("Starting sidecar inference with backend: {}", backend_name);

    // Extract config values we'll need after dropping the guard
    let embedding_model_path = config_guard.models.embedding_model_path.clone();
    let embedding_memory_mode = config_guard.embedding_memory_mode.clone();
    let inference_request = build_configured_inference_request(&config_guard);
    drop(config_guard);

    let backend_config = gateway
        .build_inference_start_config(inference_request)
        .await
        .map_err(|e| e.to_string())?;

    // Start the main LLM server
    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Started sidecar in inference mode");

    // Start embedding server for parallel modes (if embedding model is configured)
    if let Some(ref emb_path) = embedding_model_path {
        if embedding_memory_mode != EmbeddingMemoryMode::Sequential {
            let resolved_embedding_path = match resolve_embedding_model_path(emb_path) {
                Ok(path) => path,
                Err(e) => {
                    log::warn!(
                        "Failed to resolve configured embedding model path '{}': {}",
                        emb_path,
                        e
                    );
                    sync_rag_embedding_url_from_gateway(gateway.inner(), rag_manager.inner()).await;
                    return Ok(
                        synced_server_mode_info(gateway.inner(), runtime_registry.inner()).await,
                    );
                }
            };

            // Get device info for VRAM checking
            let devices = list_devices(app.clone()).await.unwrap_or_default();

            match gateway
                .start_embedding_server(
                    &resolved_embedding_path.to_string_lossy(),
                    embedding_memory_mode.clone(),
                    &devices,
                )
                .await
            {
                Ok(()) => {
                    if sync_rag_embedding_url_from_gateway(gateway.inner(), rag_manager.inner())
                        .await
                        .is_some()
                    {
                        log::info!("Embedding server started and RAG manager configured");
                    }
                }
                Err(e) => {
                    // Log but don't fail - embedding server is optional
                    log::warn!(
                        "Failed to start embedding server: {}. Vector search may not work.",
                        e
                    );
                }
            }
        } else {
            log::info!("Sequential embedding mode: embedding server will start on-demand");
        }
    }

    sync_rag_embedding_url_from_gateway(gateway.inner(), rag_manager.inner()).await;

    Ok(synced_server_mode_info(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn start_sidecar_embedding(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    rag_manager: State<'_, SharedRagManager>,
    config: State<'_, SharedAppConfig>,
) -> Result<ServerModeInfo, String> {
    let config_guard = config.read().await;
    let embedding_request = build_configured_embedding_request(&config_guard)?;
    drop(config_guard);

    let backend_config = gateway
        .build_embedding_start_config(embedding_request)
        .await
        .map_err(|e| e.to_string())?;

    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Started sidecar in embedding mode");
    sync_rag_embedding_url_from_gateway(gateway.inner(), rag_manager.inner()).await;
    Ok(synced_server_mode_info(gateway.inner(), runtime_registry.inner()).await)
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
    use inference::{ImageGenerationRequest, ImageGenerationResult, RerankRequest, RerankResponse};
    use pantograph_runtime_registry::{RuntimeRegistry, RuntimeRegistryStatus};
    use tokio::sync::mpsc;

    use crate::llm::commands::shared::synced_server_mode_info;
    use crate::llm::gateway::InferenceGateway;
    use crate::llm::startup::validate_external_server_url;
    use crate::llm::{SharedGateway, SharedRuntimeRegistry};

    struct MockProcessSpawner;

    #[async_trait]
    impl ProcessSpawner for MockProcessSpawner {
        async fn spawn_sidecar(
            &self,
            _sidecar_name: &str,
            _args: &[&str],
        ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
            Err("spawn should not be called in server command tests".to_string())
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
            "Mock backend for server command tests"
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
                "rerank should not be called in server command tests".to_string(),
            ))
        }

        async fn generate_image(
            &self,
            _request: ImageGenerationRequest,
        ) -> Result<ImageGenerationResult, BackendError> {
            Err(BackendError::Inference(
                "image generation should not be called in server command tests".to_string(),
            ))
        }
    }

    #[test]
    fn validates_external_server_urls() {
        assert_eq!(
            validate_external_server_url(" http://127.0.0.1:1234/ ").as_deref(),
            Ok("http://127.0.0.1:1234")
        );
        assert!(validate_external_server_url("").is_err());
        assert!(validate_external_server_url("ftp://127.0.0.1").is_err());
    }

    #[tokio::test]
    async fn synced_server_mode_info_refreshes_registry_before_return() {
        let gateway: SharedGateway = Arc::new(InferenceGateway::with_test_backend(
            Box::new(MockInferenceBackend::new()),
            "mock",
            Arc::new(MockProcessSpawner),
        ));
        gateway.init().await;
        gateway
            .start(&BackendConfig::default())
            .await
            .expect("gateway should start");

        let runtime_registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
        let mode_info = synced_server_mode_info(&gateway, &runtime_registry).await;

        assert_eq!(mode_info.backend_key.as_deref(), Some("mock"));

        let runtime = runtime_registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|runtime| runtime.runtime_id == "mock")
            .expect("mock runtime snapshot");
        assert_eq!(runtime.status, RuntimeRegistryStatus::Ready);
        assert!(runtime.runtime_instance_id.is_some());
    }
}
