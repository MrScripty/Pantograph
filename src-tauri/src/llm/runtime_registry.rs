//! Tauri-side re-export of backend-owned runtime-registry helpers.

pub use pantograph_embedded_runtime::runtime_registry::{
    reconcile_runtime_registry_mode_info, reconcile_runtime_registry_snapshot_override,
};
pub use pantograph_runtime_registry::{RuntimeRegistry, SharedRuntimeRegistry};
