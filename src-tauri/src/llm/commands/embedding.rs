//! Embedding server and memory mode commands.

use super::shared::SharedAppConfig;
use crate::config::EmbeddingMemoryMode;
use crate::llm::gateway::SharedGateway;
use tauri::{command, AppHandle, Manager, State};

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
