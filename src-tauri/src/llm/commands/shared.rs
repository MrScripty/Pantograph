//! Shared utilities for command modules.

use crate::config::{AppConfig, ServerModeInfo};
pub use crate::llm::runtime_registry::sync_runtime_registry_from_gateway;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};
use crate::project_root::resolve_project_root;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::constants::paths::DATA_DIR;

/// Maximum allowed size for base64-encoded images (5MB after decoding)
/// Base64 encoding increases size by ~33%, so we check for ~6.7MB encoded
pub const MAX_IMAGE_BASE64_LEN: usize = 7 * 1024 * 1024;

/// Shared app configuration
pub type SharedAppConfig = Arc<RwLock<AppConfig>>;

pub async fn synced_server_mode_info(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
) -> ServerModeInfo {
    sync_runtime_registry_from_gateway(gateway.as_ref(), runtime_registry.as_ref()).await;
    gateway.mode_info().await
}

/// Get the Pantograph project root directory.
pub fn get_project_root() -> Result<PathBuf, String> {
    resolve_project_root()
}

/// Get the project data directory for docs and RAG storage.
pub fn get_project_data_dir() -> Result<PathBuf, String> {
    let project_root = resolve_project_root()?;
    let data_dir = project_root.join(DATA_DIR);

    // Create the directory if it doesn't exist
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    Ok(data_dir)
}
