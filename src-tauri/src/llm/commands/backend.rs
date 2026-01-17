//! Backend switching and capabilities commands.

use crate::llm::backend::BackendInfo;
use crate::llm::gateway::SharedGateway;
use tauri::{command, State};

/// List all available inference backends
#[command]
pub async fn list_backends(gateway: State<'_, SharedGateway>) -> Result<Vec<BackendInfo>, String> {
    let mut backends = gateway.available_backends();
    let current_name = gateway.current_backend_name().await;

    // Mark the active backend
    for backend in &mut backends {
        backend.active = backend.name == current_name;
    }

    Ok(backends)
}

/// Get the currently active backend name
#[command]
pub async fn get_current_backend(gateway: State<'_, SharedGateway>) -> Result<String, String> {
    Ok(gateway.current_backend_name().await)
}

/// Switch to a different inference backend
///
/// Note: This stops the current backend. You'll need to call start_sidecar_inference
/// or start_sidecar_embedding to start the new backend.
#[command]
pub async fn switch_backend(
    gateway: State<'_, SharedGateway>,
    backend_name: String,
) -> Result<(), String> {
    gateway
        .switch_backend(&backend_name)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Switched to backend: {}", backend_name);
    Ok(())
}

/// Get capabilities of the current backend
#[command]
pub async fn get_backend_capabilities(
    gateway: State<'_, SharedGateway>,
) -> Result<crate::llm::backend::BackendCapabilities, String> {
    Ok(gateway.capabilities().await)
}
