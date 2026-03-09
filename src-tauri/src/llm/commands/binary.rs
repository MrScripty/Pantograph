//! Binary download and management commands.

use crate::llm::managed_binaries::{check_binary_status, download_binary, ManagedBinaryId};
use crate::llm::types::{BinaryStatus, DownloadProgress};
use tauri::{command, ipc::Channel, AppHandle};

/// Check if llama.cpp binaries are available.
#[command]
pub async fn check_llama_binaries(app: AppHandle) -> Result<BinaryStatus, String> {
    check_binary_status(&app, ManagedBinaryId::LlamaCpp).await
}

/// Download llama.cpp binaries from GitHub releases.
#[command]
pub async fn download_llama_binaries(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    download_binary(&app, ManagedBinaryId::LlamaCpp, channel).await
}

/// Check if Ollama binary is available in our managed location.
#[command]
pub async fn check_ollama_binary(app: AppHandle) -> Result<BinaryStatus, String> {
    check_binary_status(&app, ManagedBinaryId::Ollama).await
}

/// Download Ollama binary from GitHub releases.
#[command]
pub async fn download_ollama_binary(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    download_binary(&app, ManagedBinaryId::Ollama, channel).await
}
