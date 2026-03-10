//! Binary download and management commands.

use inference::{check_binary_status, download_binary, BinaryStatus, DownloadProgress, ManagedBinaryId};
use tauri::{command, ipc::Channel, AppHandle, Manager};

/// Check if llama.cpp binaries are available.
#[command]
pub async fn check_llama_binaries(app: AppHandle) -> Result<BinaryStatus, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    check_binary_status(&app_data_dir, ManagedBinaryId::LlamaCpp).await
}

/// Download llama.cpp binaries from GitHub releases.
#[command]
pub async fn download_llama_binaries(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    download_binary(&app_data_dir, ManagedBinaryId::LlamaCpp, |progress| {
        channel.send(progress).ok();
    })
    .await
}

/// Check if Ollama binary is available in our managed location.
#[command]
pub async fn check_ollama_binary(app: AppHandle) -> Result<BinaryStatus, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    check_binary_status(&app_data_dir, ManagedBinaryId::Ollama).await
}

/// Download Ollama binary from GitHub releases.
#[command]
pub async fn download_ollama_binary(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    download_binary(&app_data_dir, ManagedBinaryId::Ollama, |progress| {
        channel.send(progress).ok();
    })
    .await
}
