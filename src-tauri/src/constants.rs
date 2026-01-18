//! Application-wide constants
//!
//! Single source of truth for all magic numbers and configuration defaults.
//! This eliminates scattered hardcoded values across the codebase.

/// Network port configuration
pub mod ports {
    /// Default port for inference and embedding server
    pub const SERVER: u16 = 8080;
    /// Alternate port range start (when default is in use)
    pub const ALTERNATE_START: u16 = 8081;
    /// Number of ports to scan when looking for alternates
    pub const ALTERNATE_RANGE: u16 = 100;
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
    /// Default GPU layers (-1 = all layers on GPU)
    pub const GPU_LAYERS: i32 = -1;
    /// Default device selection
    pub const DEVICE: &str = "auto";
}

/// Device type identifiers and prefixes
pub mod device_types {
    /// Auto-select device identifier
    pub const AUTO: &str = "auto";
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
