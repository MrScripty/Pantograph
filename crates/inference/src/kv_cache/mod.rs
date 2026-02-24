//! KV Cache Management
//!
//! Generic system for saving, loading, labeling, and truncating KV caches
//! across all inference backends. The cache data is treated as opaque bytes —
//! backends handle serialization via the `KvCacheCodec` trait.

mod codec;
mod error;
mod storage;
mod store;
mod types;

pub use codec::KvCacheCodec;
pub use error::KvCacheError;
pub use storage::{DiskStorage, MemoryStorage, StorageBackend};
pub use store::KvCacheStore;
pub use types::{CacheMarker, KvCacheEntry, KvCacheMetadata, ModelFingerprint, StoragePolicy};
