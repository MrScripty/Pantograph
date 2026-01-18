// Local Tauri-specific modules
pub mod backend;
pub mod commands;
pub mod embedding_server;
pub mod gateway;
pub mod health_monitor;
pub mod port_manager;
pub mod recovery;
pub mod server;
pub mod server_discovery;
pub mod types;

// Re-export from local modules (keeping existing API)
pub use backend::BackendConfig;
pub use commands::*;
pub use gateway::{InferenceGateway, SharedGateway};
pub use port_manager::{
    check_port_available, find_available_port, kill_process, resolve_port_conflict,
    PortConflictAction, PortStatus, ProcessInfo,
};
pub use server::*;
