//! Tauri-side re-export of backend-owned runtime-registry helpers.

pub use pantograph_embedded_runtime::runtime_registry::{
    reconcile_runtime_registry_mode_info, reconcile_runtime_registry_snapshot_override,
};
pub use pantograph_runtime_registry::{RuntimeRegistry, SharedRuntimeRegistry};

pub async fn sync_runtime_registry_from_gateway(
    gateway: &crate::llm::gateway::InferenceGateway,
    registry: &RuntimeRegistry,
) {
    let mode_info = gateway.mode_info().await;
    reconcile_runtime_registry_mode_info(registry, &mode_info);
}
