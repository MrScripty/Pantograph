//! Typed extension map for non-serializable dependency injection.
//!
//! `ExecutorExtensions` allows hosts to inject runtime objects (like API clients,
//! database handles, etc.) into the workflow execution pipeline. Extensions are
//! threaded through the `TaskExecutor` → `NodeExecutor` chain, making them
//! available to any node during execution.
//!
//! # Example
//!
//! ```ignore
//! use node_engine::ExecutorExtensions;
//! use std::sync::Arc;
//!
//! let mut ext = ExecutorExtensions::new();
//! ext.set("my_service", Arc::new(MyService::new()));
//!
//! // In a NodeExecutor:
//! if let Some(svc) = extensions.get::<Arc<MyService>>("my_service") {
//!     svc.do_something().await;
//! }
//! ```

use std::any::Any;
use std::collections::HashMap;

/// Typed extension map for injecting non-serializable dependencies
/// into workflow execution.
///
/// Unlike `graph_flow::Context` which stores `serde_json::Value`, this map
/// holds arbitrary `Send + Sync` types via `Box<dyn Any>`. This is used for
/// runtime objects that cannot be serialized (API clients, database handles, etc.).
pub struct ExecutorExtensions {
    inner: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl ExecutorExtensions {
    /// Create an empty extension map.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Insert a typed value under the given key.
    ///
    /// If a value already exists for this key, it is replaced.
    pub fn set<T: Send + Sync + 'static>(&mut self, key: &str, value: T) {
        self.inner.insert(key.to_string(), Box::new(value));
    }

    /// Get a reference to a typed value by key.
    ///
    /// Returns `None` if the key doesn't exist or the type doesn't match.
    pub fn get<T: Send + Sync + 'static>(&self, key: &str) -> Option<&T> {
        self.inner.get(key).and_then(|v| v.downcast_ref())
    }

    /// Check whether a key exists in the map.
    pub fn has(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }
}

impl Default for ExecutorExtensions {
    fn default() -> Self {
        Self::new()
    }
}

/// Well-known extension keys for standard dependencies.
pub mod extension_keys {
    /// Key for `Arc<pumas_library::PumasApi>` — model library access.
    pub const PUMAS_API: &str = "pumas_api";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_set_and_get() {
        let mut ext = ExecutorExtensions::new();
        ext.set("name", "hello".to_string());

        assert_eq!(ext.get::<String>("name"), Some(&"hello".to_string()));
        assert!(ext.has("name"));
        assert!(!ext.has("missing"));
    }

    #[test]
    fn test_type_mismatch_returns_none() {
        let mut ext = ExecutorExtensions::new();
        ext.set("count", 42u32);

        // Wrong type
        assert!(ext.get::<String>("count").is_none());
        // Correct type
        assert_eq!(ext.get::<u32>("count"), Some(&42));
    }

    #[test]
    fn test_arc_values() {
        let mut ext = ExecutorExtensions::new();
        let value = Arc::new(vec![1, 2, 3]);
        ext.set("data", value.clone());

        let retrieved = ext.get::<Arc<Vec<i32>>>("data").unwrap();
        assert_eq!(retrieved.as_ref(), &vec![1, 2, 3]);
    }

    #[test]
    fn test_replace_value() {
        let mut ext = ExecutorExtensions::new();
        ext.set("key", "first".to_string());
        ext.set("key", "second".to_string());

        assert_eq!(ext.get::<String>("key"), Some(&"second".to_string()));
    }
}
