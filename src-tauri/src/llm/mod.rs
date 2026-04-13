// Local Tauri-specific modules
pub mod backend;
pub mod commands;
pub mod gateway;
pub mod health_monitor;
pub mod port_manager;
pub mod process_tauri;
pub mod recovery;
pub mod server_discovery;
pub mod startup;
pub mod types;

// Re-export from local modules (keeping existing API)
pub use backend::{EmbeddingStartRequest, InferenceStartRequest};
pub use commands::*;
pub use gateway::{InferenceGateway, SharedGateway};
