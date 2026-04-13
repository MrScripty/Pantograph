//! LLM server lifecycle management commands.

use super::config::list_devices;
use super::shared::SharedAppConfig;
use crate::agent::rag::SharedRagManager;
use crate::config::{EmbeddingMemoryMode, ServerModeInfo};
use crate::llm::gateway::SharedGateway;
use crate::llm::startup::{
    build_configured_embedding_request, build_configured_inference_request,
    build_explicit_llamacpp_inference_request, build_external_inference_request,
    resolve_embedding_model_path,
};
use tauri::{command, AppHandle, State};

#[command]
pub async fn connect_to_server(
    gateway: State<'_, SharedGateway>,
    url: String,
) -> Result<ServerModeInfo, String> {
    if gateway.mode_info().await.backend_key.as_deref() != Some("llama_cpp") {
        gateway
            .switch_backend("llama_cpp")
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

    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_llm(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
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

    Ok(gateway.mode_info().await)
}

#[command]
pub async fn get_llm_status(gateway: State<'_, SharedGateway>) -> Result<ServerModeInfo, String> {
    Ok(gateway.mode_info().await)
}

#[command]
pub async fn stop_llm(gateway: State<'_, SharedGateway>) -> Result<ServerModeInfo, String> {
    gateway.stop().await;
    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_inference(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
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
                    return Ok(gateway.mode_info().await);
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
                    // Set embedding URL in RAG manager so search() will work
                    if let Some(url) = gateway.embedding_url().await {
                        let mut rag = rag_manager.write().await;
                        rag.set_embedding_url(url);
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

    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_embedding(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
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
    Ok(gateway.mode_info().await)
}

#[cfg(test)]
mod tests {
    use crate::llm::startup::validate_external_server_url;

    #[test]
    fn validates_external_server_urls() {
        assert_eq!(
            validate_external_server_url(" http://127.0.0.1:1234/ ").as_deref(),
            Ok("http://127.0.0.1:1234")
        );
        assert!(validate_external_server_url("").is_err());
        assert!(validate_external_server_url("ftp://127.0.0.1").is_err());
    }
}
