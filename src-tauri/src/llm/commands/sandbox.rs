//! Sandbox configuration commands.

use super::shared::SharedAppConfig;
use crate::config::SandboxConfig;
use serde::Serialize;
use std::path::PathBuf;
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

/// Result of component validation
#[derive(Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub error: Option<String>,
}

/// Get the project root directory.
/// Uses CARGO_MANIFEST_DIR at compile time, parent of src-tauri.
fn get_project_root_internal() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent")
        .to_path_buf()
}

/// Validate a generated component file.
/// Called by frontend before importing to catch invalid imports/syntax.
///
/// The `relative_path` should be relative to project root (e.g., "/src/generated/MyComponent.svelte")
/// and the backend will resolve the actual filesystem path.
#[command]
pub async fn validate_component(
    relative_path: String,
) -> Result<ValidationResult, String> {
    let project_root = get_project_root_internal();
    let script_path = project_root.join("scripts").join("validate-esbuild.mjs");

    if !script_path.exists() {
        return Err(format!(
            "Validation script not found: {}",
            script_path.display()
        ));
    }

    // Convert relative path to absolute
    // Remove leading slash if present for path joining
    let clean_relative = relative_path.trim_start_matches('/');
    let file_path = project_root.join(clean_relative);

    let output = tokio::process::Command::new("node")
        .arg(&script_path)
        .arg(&file_path)
        .arg(&project_root)
        .output()
        .await
        .map_err(|e| format!("Failed to run validation script: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON result from validation script
    let result: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
        format!(
            "Failed to parse validation result: {}. Output: {}",
            e, stdout
        )
    })?;

    let valid = result
        .get("valid")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let error = result
        .get("error")
        .and_then(|e| e.as_str())
        .map(String::from);

    Ok(ValidationResult { valid, error })
}
