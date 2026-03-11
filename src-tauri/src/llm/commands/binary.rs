//! Runtime download and management commands.

use inference::{
    download_binary, list_binary_capabilities, remove_binary, DownloadProgress,
    ManagedBinaryCapability, ManagedBinaryId,
};
use tauri::{command, ipc::Channel, AppHandle, Manager};

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))
}

/// List managed runtime capabilities and allowed actions for the current platform.
#[command]
pub async fn list_managed_runtimes(app: AppHandle) -> Result<Vec<ManagedBinaryCapability>, String> {
    let app_data_dir = app_data_dir(&app)?;
    list_binary_capabilities(&app_data_dir)
}

/// Install one managed runtime into the app-owned runtime directory.
#[command]
pub async fn install_managed_runtime(
    app: AppHandle,
    binary_id: ManagedBinaryId,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    let app_data_dir = app_data_dir(&app)?;
    download_binary(&app_data_dir, binary_id, |progress| {
        let _ = channel.send(progress);
    })
    .await
}

/// Remove one managed runtime from the app-owned runtime directory.
#[command]
pub async fn remove_managed_runtime(
    app: AppHandle,
    binary_id: ManagedBinaryId,
) -> Result<(), String> {
    let app_data_dir = app_data_dir(&app)?;
    remove_binary(&app_data_dir, binary_id).await
}
