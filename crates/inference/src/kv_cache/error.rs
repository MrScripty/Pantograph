//! KV cache error types

/// Errors that can occur during KV cache operations.
#[derive(Debug, thiserror::Error)]
pub enum KvCacheError {
    #[error("cache not found: {cache_id}")]
    NotFound { cache_id: String },

    #[error("model mismatch: cache is for '{cache_model}', requested '{requested_model}'")]
    ModelMismatch {
        cache_model: String,
        requested_model: String,
    },

    #[error("runtime mismatch: cache is for '{cache_runtime}', requested '{requested_runtime}'")]
    RuntimeMismatch {
        cache_runtime: String,
        requested_runtime: String,
    },

    #[error("cache '{cache_id}' is missing a runtime fingerprint and cannot be reused as an executable handle")]
    MissingRuntimeFingerprint { cache_id: String },

    #[error("marker not found: {marker_name}")]
    MarkerNotFound { marker_name: String },

    #[error("storage error: {source}")]
    Storage {
        #[from]
        source: std::io::Error,
    },

    #[error("codec error: {message}")]
    Codec { message: String },

    #[error("invalid data: {message}")]
    InvalidData { message: String },
}
