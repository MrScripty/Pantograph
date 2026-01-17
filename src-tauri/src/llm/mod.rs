// Local Tauri-specific modules (these will continue to use local implementations
// while gradually migrating to the inference library)
pub mod backend;
pub mod commands; // Now a directory with domain-specific modules
pub mod device;
pub mod embedding_server;
pub mod gateway;
pub mod process_tauri;
pub mod server;
pub mod types;

// Re-export from local modules (keeping existing API)
pub use backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendInfo, BackendRegistry,
    InferenceBackend, LlamaCppBackend,
};
pub use commands::*;
pub use device::*;
pub use embedding_server::EmbeddingServer;
pub use gateway::{GatewayError, InferenceGateway, SharedGateway};
pub use server::*;
pub use types::*;

// Re-export Tauri-specific spawner (for future use)
pub use process_tauri::{create_spawner, TauriProcessSpawner};

// Re-export shared types from inference library that don't conflict
// (These can be used by external code that wants library types)
pub mod lib_types {
    pub use inference::{
        DeviceBackend as LibDeviceBackend,
        DeviceConfig as LibDeviceConfig,
        EmbeddingMemoryMode as LibEmbeddingMemoryMode,
        ProcessEvent, ProcessHandle, ProcessSpawner,
    };
}
