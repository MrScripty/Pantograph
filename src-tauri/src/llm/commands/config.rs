//! Model, device, and app configuration commands.

use super::shared::SharedAppConfig;
use crate::config::{AppConfig, DeviceConfig, DeviceInfo, ModelConfig};
use crate::workflow::commands::SharedWorkflowService;
use inference::list_llamacpp_devices;
use tauri::{AppHandle, Manager, State, command};

#[command]
pub async fn get_model_config(config: State<'_, SharedAppConfig>) -> Result<ModelConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.models.clone())
}

#[command]
pub async fn set_model_config(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    models: ModelConfig,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    config_guard.models = models;
    config_guard
        .save(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Model configuration saved");
    Ok(())
}

#[command]
pub async fn get_app_config(config: State<'_, SharedAppConfig>) -> Result<AppConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.clone())
}

#[command]
pub async fn set_app_config(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    workflow_service: State<'_, SharedWorkflowService>,
    new_config: AppConfig,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    *config_guard = new_config;
    let max_loaded_sessions = config_guard.workflow.max_loaded_sessions;
    config_guard
        .save(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;
    workflow_service
        .set_loaded_runtime_capacity_limit(max_loaded_sessions)
        .map_err(|e| format!("Failed to apply workflow runtime config: {}", e))?;

    log::info!("Application configuration saved");
    Ok(())
}

#[command]
pub async fn get_device_config(config: State<'_, SharedAppConfig>) -> Result<DeviceConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.device.clone())
}

#[command]
pub async fn set_device_config(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    device: DeviceConfig,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    config_guard.device = device;
    config_guard
        .save(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Device configuration saved");
    Ok(())
}

/// List available compute devices by running llama-server --list-devices
#[command]
pub async fn list_devices(app: AppHandle) -> Result<Vec<DeviceInfo>, String> {
    log::info!("Listing available devices...");
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let devices = list_llamacpp_devices(&app_data_dir).await?;
    log::info!("Found {} devices", devices.len());
    Ok(devices
        .into_iter()
        .map(|device| DeviceInfo {
            id: device.id,
            name: device.name,
            total_vram_mb: device.total_vram_mb,
            free_vram_mb: device.free_vram_mb,
        })
        .collect())
}
