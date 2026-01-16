//! Application-wide constants
//!
//! Single source of truth for all magic numbers and configuration defaults.
//! This eliminates scattered hardcoded values across the codebase.

/// Network port configuration
pub mod ports {
    /// Default port for inference and embedding server
    pub const SERVER: u16 = 8080;
}

/// Timeout configuration (in seconds)
pub mod timeouts {
    /// Maximum time to wait for server startup
    pub const SERVER_STARTUP_SECS: u64 = 120;
}

/// Default values for inference configuration
pub mod defaults {
    /// Default context window size
    pub const CONTEXT_SIZE: u32 = 8192;
    /// Default maximum tokens for generation
    pub const MAX_TOKENS: u32 = 2048;
    /// Default GPU layers (-1 = all layers on GPU)
    pub const GPU_LAYERS: i32 = -1;
    /// Default device selection
    pub const DEVICE: &str = "auto";
}

/// Device type identifiers and prefixes
pub mod device_types {
    /// CPU-only mode identifier
    pub const CPU: &str = "none";
    /// Auto-select device identifier
    pub const AUTO: &str = "auto";
    /// CUDA device prefix (e.g., "CUDA0", "CUDA1")
    pub const CUDA_PREFIX: &str = "CUDA";
    /// Vulkan device prefix (e.g., "Vulkan0", "Vulkan1")
    pub const VULKAN_PREFIX: &str = "Vulkan";
    /// Metal device prefix (e.g., "Metal0")
    pub const METAL_PREFIX: &str = "Metal";
}

/// Server host configuration
pub mod hosts {
    /// Default host for local server binding
    pub const LOCAL: &str = "127.0.0.1";
}

/// Data storage paths
pub mod paths {
    /// Directory for downloaded/generated data (svelte docs, vector embeddings, etc.)
    /// This is relative to the project root and should be gitignored.
    pub const DATA_DIR: &str = "data";
}
