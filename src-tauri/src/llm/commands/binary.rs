//! Thin Tauri transport for the backend-owned managed-runtime manager.

use pantograph_embedded_runtime::{
    cancel_managed_runtime_manager_job, inspect_managed_runtime_manager_runtime,
    install_managed_runtime_manager_runtime, list_managed_runtime_manager_runtimes,
    remove_managed_runtime_manager_runtime, select_managed_runtime_manager_version,
    set_default_managed_runtime_manager_version_view, ManagedRuntimeManagerProgress,
    ManagedRuntimeManagerRuntimeView,
};
use tauri::{command, ipc::Channel, AppHandle, Manager};

use inference::ManagedBinaryId;

fn app_data_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Failed to get app data dir: {error}"))
}

/// List backend-owned managed-runtime view state for the current platform.
#[command]
pub async fn list_managed_runtimes(
    app: AppHandle,
) -> Result<Vec<ManagedRuntimeManagerRuntimeView>, String> {
    let app_data_dir = app_data_dir(&app)?;
    list_managed_runtime_manager_runtimes(&app_data_dir)
}

/// Inspect one managed runtime, including version, readiness, and history state.
#[command]
pub async fn inspect_managed_runtime(
    app: AppHandle,
    binary_id: ManagedBinaryId,
) -> Result<ManagedRuntimeManagerRuntimeView, String> {
    let app_data_dir = app_data_dir(&app)?;
    inspect_managed_runtime_manager_runtime(&app_data_dir, binary_id)
}

/// Install one managed runtime into the app-owned runtime directory.
#[command]
pub async fn install_managed_runtime(
    app: AppHandle,
    binary_id: ManagedBinaryId,
    channel: Channel<ManagedRuntimeManagerProgress>,
) -> Result<(), String> {
    let app_data_dir = app_data_dir(&app)?;
    install_managed_runtime_manager_runtime(&app_data_dir, binary_id, |progress| {
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
    remove_managed_runtime_manager_runtime(&app_data_dir, binary_id).await
}

/// Request cancellation for the active managed runtime install job.
#[command]
pub async fn cancel_managed_runtime_job(
    app: AppHandle,
    binary_id: ManagedBinaryId,
) -> Result<(), String> {
    let app_data_dir = app_data_dir(&app)?;
    cancel_managed_runtime_manager_job(&app_data_dir, binary_id)
}

/// Update the selected runtime version for launch-time command resolution.
#[command]
pub async fn select_managed_runtime_version(
    app: AppHandle,
    binary_id: ManagedBinaryId,
    version: Option<String>,
) -> Result<ManagedRuntimeManagerRuntimeView, String> {
    let app_data_dir = app_data_dir(&app)?;
    select_managed_runtime_manager_version(&app_data_dir, binary_id, version.as_deref())
}

/// Update the default runtime version that future selections inherit from.
#[command]
pub async fn set_default_managed_runtime_version(
    app: AppHandle,
    binary_id: ManagedBinaryId,
    version: Option<String>,
) -> Result<ManagedRuntimeManagerRuntimeView, String> {
    let app_data_dir = app_data_dir(&app)?;
    set_default_managed_runtime_manager_version_view(&app_data_dir, binary_id, version.as_deref())
}
