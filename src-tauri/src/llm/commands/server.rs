//! LLM server lifecycle management commands.

use super::config::list_devices;
use super::shared::SharedAppConfig;
use crate::agent::rag::SharedRagManager;
use crate::config::{EmbeddingMemoryMode, ServerModeInfo};
use crate::llm::gateway::SharedGateway;
use crate::llm::types::LLMStatus;
use crate::llm::BackendConfig;
use tauri::{command, AppHandle, State};

#[command]
pub async fn connect_to_server(
    _gateway: State<'_, SharedGateway>,
    _url: String,
) -> Result<LLMStatus, String> {
    // TODO: Implement connect_external through gateway interface
    // For now, this feature is disabled during the gateway migration
    Err("External server connection not yet supported through gateway".to_string())
}

#[command]
pub async fn start_sidecar_llm(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    model_path: String,
    mmproj_path: String,
) -> Result<LLMStatus, String> {
    let config_guard = config.read().await;
    let device = config_guard.device.clone();
    drop(config_guard);

    let backend_config = BackendConfig {
        model_path: Some(std::path::PathBuf::from(&model_path)),
        mmproj_path: Some(std::path::PathBuf::from(&mmproj_path)),
        device: Some(device.device),
        gpu_layers: Some(device.gpu_layers),
        embedding_mode: false,
        ..Default::default()
    };

    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LLMStatus {
        ready: gateway.is_ready().await,
        mode: "sidecar_inference".to_string(),
        url: gateway.base_url().await,
    })
}

#[command]
pub async fn get_llm_status(gateway: State<'_, SharedGateway>) -> Result<LLMStatus, String> {
    let ready = gateway.is_ready().await;
    let url = gateway.base_url().await;
    let backend_name = gateway.current_backend_name().await;

    Ok(LLMStatus {
        ready,
        mode: if ready {
            format!("sidecar_{}", backend_name)
        } else {
            "none".to_string()
        },
        url,
    })
}

#[command]
pub async fn stop_llm(gateway: State<'_, SharedGateway>) -> Result<(), String> {
    gateway.stop().await;
    Ok(())
}

#[command]
pub async fn get_server_mode(gateway: State<'_, SharedGateway>) -> Result<ServerModeInfo, String> {
    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_inference(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
) -> Result<ServerModeInfo, String> {
    let backend_name = gateway.current_backend_name().await;
    log::info!(
        "Starting sidecar inference with backend: {}",
        backend_name
    );

    let config_guard = config.read().await;

    // Extract config values we'll need after dropping the guard
    let embedding_model_path = config_guard.models.embedding_model_path.clone();
    let embedding_memory_mode = config_guard.embedding_memory_mode.clone();

    // Build backend-specific config
    let backend_config = match backend_name.as_str() {
        "Ollama" => {
            // Ollama uses model names, not file paths
            let model_name = config_guard.models.ollama_vlm_model.as_ref()
                .ok_or_else(|| "Ollama VLM model not configured. Set a model like 'llava:13b' or 'qwen2-vl:7b' in Model Configuration.".to_string())?;
            BackendConfig {
                model_name: Some(model_name.clone()),
                embedding_mode: false,
                ..Default::default()
            }
        }
        _ => {
            // llama.cpp and others use file paths
            let model_path = config_guard
                .models
                .vlm_model_path
                .as_ref()
                .ok_or_else(|| "VLM model path not configured".to_string())?;
            let mmproj_path = config_guard
                .models
                .vlm_mmproj_path
                .as_ref()
                .ok_or_else(|| "VLM mmproj path not configured".to_string())?;

            BackendConfig {
                model_path: Some(std::path::PathBuf::from(model_path)),
                mmproj_path: Some(std::path::PathBuf::from(mmproj_path)),
                device: Some(config_guard.device.device.clone()),
                gpu_layers: Some(config_guard.device.gpu_layers),
                embedding_mode: false,
                ..Default::default()
            }
        }
    };
    drop(config_guard);

    // Start the main LLM server
    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Started sidecar in inference mode");

    // Start embedding server for parallel modes (if embedding model is configured)
    if let Some(ref emb_path) = embedding_model_path {
        if embedding_memory_mode != EmbeddingMemoryMode::Sequential {
            // Get device info for VRAM checking
            let devices = list_devices(app.clone()).await.unwrap_or_default();

            match gateway
                .start_embedding_server(emb_path, embedding_memory_mode.clone(), &devices)
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

    let model_path = config_guard
        .models
        .embedding_model_path
        .as_ref()
        .ok_or_else(|| "Embedding model path not configured".to_string())?;

    let backend_config = BackendConfig {
        model_path: Some(std::path::PathBuf::from(model_path)),
        device: Some(config_guard.device.device.clone()),
        gpu_layers: Some(config_guard.device.gpu_layers),
        embedding_mode: true,
        ..Default::default()
    };
    drop(config_guard);

    gateway
        .start(&backend_config)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Started sidecar in embedding mode");
    Ok(gateway.mode_info().await)
}
