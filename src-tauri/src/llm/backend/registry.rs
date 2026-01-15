//! Backend registry for runtime backend discovery and instantiation
//!
//! The registry manages available backends and provides factory methods
//! to create backend instances. Backends are registered at compile time
//! via feature flags.

use std::collections::HashMap;

use super::{BackendError, BackendInfo, InferenceBackend, LlamaCppBackend};
#[cfg(feature = "backend-ollama")]
use super::OllamaBackend;
#[cfg(feature = "backend-candle")]
use super::CandleBackend;

/// Factory trait for creating backend instances
pub trait BackendFactory: Send + Sync {
    /// Create a new backend instance
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError>;

    /// Get information about this backend
    fn info(&self) -> BackendInfo;
}

/// Factory for llama.cpp backend
pub struct LlamaCppFactory;

impl BackendFactory for LlamaCppFactory {
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError> {
        Ok(Box::new(LlamaCppBackend::new()))
    }

    fn info(&self) -> BackendInfo {
        BackendInfo {
            name: "llama.cpp".to_string(),
            description: "Local llama.cpp server with GGUF model support".to_string(),
            capabilities: LlamaCppBackend::static_capabilities(),
            active: false,
            available: true,
            unavailable_reason: None,
            can_install: true, // Binaries can be downloaded from GitHub releases
        }
    }
}

/// Factory for Ollama backend
#[cfg(feature = "backend-ollama")]
pub struct OllamaFactory;

#[cfg(feature = "backend-ollama")]
impl BackendFactory for OllamaFactory {
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError> {
        Ok(Box::new(OllamaBackend::new()))
    }

    fn info(&self) -> BackendInfo {
        let (available, unavailable_reason) = OllamaBackend::check_availability();
        BackendInfo {
            name: "Ollama".to_string(),
            description: "Ollama daemon with automatic model management".to_string(),
            capabilities: OllamaBackend::static_capabilities(),
            active: false,
            available,
            unavailable_reason,
            can_install: OllamaBackend::can_auto_install(),
        }
    }
}

/// Factory for Candle backend
#[cfg(feature = "backend-candle")]
pub struct CandleFactory;

#[cfg(feature = "backend-candle")]
impl BackendFactory for CandleFactory {
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError> {
        Ok(Box::new(CandleBackend::new()))
    }

    fn info(&self) -> BackendInfo {
        let (available, unavailable_reason) = CandleBackend::check_availability();
        BackendInfo {
            name: "Candle".to_string(),
            description: if available {
                "In-process Candle inference (CUDA)".to_string()
            } else {
                "In-process Candle inference (CUDA required)".to_string()
            },
            capabilities: CandleBackend::static_capabilities(),
            active: false,
            available,
            unavailable_reason,
            can_install: false, // CUDA must be installed system-wide, can't auto-install
        }
    }
}

/// Registry of available inference backends
///
/// Backends are registered at compile time based on feature flags.
/// At runtime, the registry can list available backends and create
/// instances on demand.
pub struct BackendRegistry {
    factories: HashMap<String, Box<dyn BackendFactory>>,
}

impl BackendRegistry {
    /// Create a new registry with all available backends registered
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };

        // Always register llama.cpp (default backend)
        registry.register("llama.cpp", Box::new(LlamaCppFactory));

        // Register Ollama backend if feature is enabled
        #[cfg(feature = "backend-ollama")]
        registry.register("Ollama", Box::new(OllamaFactory));

        // Register Candle backend if feature is enabled
        #[cfg(feature = "backend-candle")]
        registry.register("Candle", Box::new(CandleFactory));

        registry
    }

    /// Register a backend factory
    pub fn register(&mut self, name: &str, factory: Box<dyn BackendFactory>) {
        self.factories.insert(name.to_string(), factory);
    }

    /// List all available backend names
    pub fn available_names(&self) -> Vec<&str> {
        self.factories.keys().map(|s| s.as_str()).collect()
    }

    /// Get information about all registered backends
    pub fn list(&self) -> Vec<BackendInfo> {
        self.factories.values().map(|f| f.info()).collect()
    }

    /// Create a backend instance by name
    pub fn create(&self, name: &str) -> Result<Box<dyn InferenceBackend>, BackendError> {
        self.factories
            .get(name)
            .ok_or_else(|| BackendError::Config(format!("Unknown backend: {}", name)))?
            .create()
    }

    /// Check if a backend is available
    pub fn is_available(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_llamacpp() {
        let registry = BackendRegistry::new();
        assert!(registry.is_available("llama.cpp"));
    }

    #[test]
    fn test_registry_list() {
        let registry = BackendRegistry::new();
        let backends = registry.list();
        assert!(!backends.is_empty());
        assert!(backends.iter().any(|b| b.name == "llama.cpp"));
    }
}
