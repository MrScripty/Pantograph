//! Shared utilities for command modules.

use crate::config::AppConfig;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::constants::paths::DATA_DIR;

/// Maximum allowed size for base64-encoded images (5MB after decoding)
/// Base64 encoding increases size by ~33%, so we check for ~6.7MB encoded
pub const MAX_IMAGE_BASE64_LEN: usize = 7 * 1024 * 1024;

/// Shared app configuration
pub type SharedAppConfig = Arc<RwLock<AppConfig>>;

/// Get the project data directory for docs and RAG storage.
/// Uses CARGO_MANIFEST_DIR (src-tauri/) and goes up one level to get project root.
/// This ensures the data directory is at the project root regardless of the
/// current working directory (which varies during `tauri dev`).
pub fn get_project_data_dir() -> Result<PathBuf, String> {
    // CARGO_MANIFEST_DIR is set at compile time to the directory containing Cargo.toml (src-tauri/)
    // We go up one level to get the actual project root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .ok_or_else(|| "Failed to get project root from CARGO_MANIFEST_DIR".to_string())?;

    let data_dir = project_root.join(DATA_DIR);

    // Create the directory if it doesn't exist
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    Ok(data_dir)
}
