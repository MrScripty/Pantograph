//! Backend-specific KV cache codec trait
//!
//! Each inference backend implements this trait to handle its native
//! cache format. The generic KV cache store delegates format-specific
//! operations (truncation, fingerprinting) through this trait.

use super::error::KvCacheError;
use super::types::ModelFingerprint;

/// Codec for backend-specific KV cache serialization and manipulation.
///
/// Backends implement this to handle their native cache format while
/// keeping the store decoupled from backend internals.
pub trait KvCacheCodec: Send + Sync {
    /// Truncate cache data to keep only tokens up to `token_position`.
    fn truncate(&self, data: &[u8], token_position: usize) -> Result<Vec<u8>, KvCacheError>;

    /// Compute fingerprint for the currently loaded model.
    fn model_fingerprint(&self) -> Result<ModelFingerprint, KvCacheError>;

    /// Backend identifier string (e.g. "pytorch", "llamacpp").
    fn backend_name(&self) -> &'static str;
}
