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

/// Default values for inference configuration
pub mod defaults {
    /// Default GPU layers (-1 = all layers on GPU)
    pub const GPU_LAYERS: i32 = -1;
    /// Default device selection
    pub const DEVICE: &str = "auto";
}

/// Data storage paths
pub mod paths {
    /// Directory for downloaded/generated data (svelte docs, vector embeddings, etc.)
    /// This is relative to the project root and should be gitignored.
    pub const DATA_DIR: &str = "data";
}
