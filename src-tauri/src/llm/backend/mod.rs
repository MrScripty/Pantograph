//! Inference backend types — re-exported from the `inference` crate.
//!
//! The local backend trait and implementations have been replaced by the
//! crate versions which use the `ProcessSpawner` abstraction instead of
//! requiring a Tauri `AppHandle`.

// Re-export backend types used by the Tauri app
pub use inference::backend::{BackendCapabilities, BackendConfig, BackendInfo};
