//! Model, device, and app configuration commands.

use super::shared::SharedAppConfig;
use crate::config::{AppConfig, DeviceConfig, DeviceInfo, ModelConfig};
use tauri::{command, AppHandle, Manager, State};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

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
    new_config: AppConfig,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    *config_guard = new_config;
    config_guard
        .save(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

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

    // Run llama-server with --list-devices flag
    // Use --device CUDA0 to trigger the CUDA binary which shows all device types
    let (mut rx, _child) = app
        .shell()
        .sidecar("llama-server-wrapper")
        .map_err(|e| format!("Failed to create sidecar: {}", e))?
        .args(["--device", "CUDA0", "--list-devices"])
        .spawn()
        .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

    // Collect output
    let mut output = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => {
                if let Ok(text) = String::from_utf8(line) {
                    output.push_str(&text);
                }
            }
            CommandEvent::Stderr(line) => {
                if let Ok(text) = String::from_utf8(line) {
                    output.push_str(&text);
                }
            }
            CommandEvent::Terminated(_) => break,
            _ => {}
        }
    }

    log::info!("Device list output: {}", output);

    // Parse the output
    // Format: "  Vulkan0: Intel(R) Graphics (RPL-P) (32003 MiB, 28803 MiB free)"
    let mut devices = Vec::new();

    // Always add CPU option first
    devices.push(DeviceInfo {
        id: "none".to_string(),
        name: "CPU Only".to_string(),
        total_vram_mb: 0,
        free_vram_mb: 0,
    });

    for line in output.lines() {
        let line = line.trim();
        // Look for lines like "Vulkan0: ..." or "CUDA0: ..."
        if let Some(colon_pos) = line.find(':') {
            let id = line[..colon_pos].trim();
            // Skip if it doesn't look like a device ID (e.g., "Available devices")
            if !id.contains(' ')
                && (id.starts_with("Vulkan")
                    || id.starts_with("CUDA")
                    || id.starts_with("Metal"))
            {
                let rest = line[colon_pos + 1..].trim();

                // Parse name and VRAM info
                // Format: "NVIDIA GeForce RTX 4060 Laptop GPU (8188 MiB, 547 MiB free)"
                let (name, total_vram, free_vram) = if let Some(paren_pos) = rest.rfind('(') {
                    let name = rest[..paren_pos].trim();
                    let vram_info = &rest[paren_pos + 1..].trim_end_matches(')');

                    // Parse "8188 MiB, 547 MiB free"
                    let parts: Vec<&str> = vram_info.split(',').collect();
                    let total = parts
                        .first()
                        .and_then(|s| s.trim().strip_suffix(" MiB"))
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    let free = parts
                        .get(1)
                        .and_then(|s| s.trim().strip_suffix(" MiB free"))
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);

                    (name.to_string(), total, free)
                } else {
                    (rest.to_string(), 0, 0)
                };

                devices.push(DeviceInfo {
                    id: id.to_string(),
                    name,
                    total_vram_mb: total_vram,
                    free_vram_mb: free_vram,
                });
            }
        }
    }

    log::info!("Found {} devices", devices.len());
    Ok(devices)
}
