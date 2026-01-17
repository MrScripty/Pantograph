//! Sandbox configuration commands.

use super::shared::SharedAppConfig;
use crate::config::SandboxConfig;
use tauri::{command, AppHandle, Manager, State};

/// Get the current sandbox configuration
#[command]
pub async fn get_sandbox_config(
    config: State<'_, SharedAppConfig>,
) -> Result<SandboxConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.sandbox.clone())
}

/// Set the sandbox configuration
#[command]
pub async fn set_sandbox_config(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    sandbox: SandboxConfig,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    config_guard.sandbox = sandbox;
    config_guard
        .save(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Sandbox configuration saved");
    Ok(())
}

/// Get the current system prompt
#[command]
pub async fn get_system_prompt() -> Result<String, String> {
    Ok(crate::agent::prompt::SYSTEM_PROMPT.to_string())
}

/// Set the system prompt (saves to prompt.rs)
/// Note: This modifies the source file directly. Changes take effect on next agent run.
#[command]
pub async fn set_system_prompt(content: String) -> Result<(), String> {
    // Get the path to prompt.rs
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let prompt_path = std::path::Path::new(manifest_dir)
        .join("src")
        .join("agent")
        .join("prompt.rs");

    // Build the file content using string concatenation to avoid raw string nesting issues
    let mut new_content = String::new();
    new_content.push_str("/// System prompt for the UI generation agent\n");
    new_content.push_str("pub const SYSTEM_PROMPT: &str = r##\"\n");
    new_content.push_str(&content);
    new_content.push_str("\n\"##;\n");

    // Write the new content
    std::fs::write(&prompt_path, &new_content)
        .map_err(|e| format!("Failed to write prompt file: {}", e))?;

    log::info!("System prompt saved to {:?}", prompt_path);
    Ok(())
}
