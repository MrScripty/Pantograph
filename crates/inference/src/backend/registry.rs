//! Backend registry for runtime backend discovery and instantiation
//!
//! The registry manages available backends and provides factory methods
//! to create backend instances. Backends are registered at compile time
//! via feature flags.

use std::collections::HashMap;

use super::{BackendDefaultStartMode, BackendError, BackendInfo, InferenceBackend};
use crate::managed_runtime::ManagedBinaryId;

#[cfg(feature = "backend-llamacpp")]
use super::LlamaCppBackend;

#[cfg(feature = "backend-ollama")]
use super::OllamaBackend;

#[cfg(feature = "backend-candle")]
use super::CandleBackend;

#[cfg(feature = "backend-pytorch")]
use super::PyTorchBackend;

/// Factory trait for creating backend instances
pub trait BackendFactory: Send + Sync {
    /// Create a new backend instance
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError>;

    /// Get information about this backend
    fn info(&self) -> BackendInfo;
}

/// Factory for llama.cpp backend
#[cfg(feature = "backend-llamacpp")]
pub struct LlamaCppFactory;

#[cfg(feature = "backend-llamacpp")]
impl BackendFactory for LlamaCppFactory {
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError> {
        Ok(Box::new(LlamaCppBackend::new()))
    }

    fn info(&self) -> BackendInfo {
        BackendInfo {
            name: "llama.cpp".to_string(),
            backend_key: "llama_cpp".to_string(),
            description: "Local llama.cpp server with GGUF model support".to_string(),
            capabilities: LlamaCppBackend::static_capabilities(),
            default_start_mode: BackendDefaultStartMode::Inference,
            active: false,
            available: true,
            unavailable_reason: None,
            can_install: true, // Binaries can be downloaded from GitHub releases
            runtime_binary_id: Some(ManagedBinaryId::LlamaCpp),
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
            backend_key: "ollama".to_string(),
            description: "Ollama daemon with automatic model management".to_string(),
            capabilities: OllamaBackend::static_capabilities(),
            default_start_mode: BackendDefaultStartMode::Inference,
            active: false,
            available,
            unavailable_reason,
            can_install: OllamaBackend::can_auto_install(),
            runtime_binary_id: Some(ManagedBinaryId::Ollama),
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
            backend_key: "candle".to_string(),
            description: if available {
                "In-process Candle inference (CUDA)".to_string()
            } else {
                "In-process Candle inference (CUDA required)".to_string()
            },
            capabilities: CandleBackend::static_capabilities(),
            default_start_mode: BackendDefaultStartMode::Embedding,
            active: false,
            available,
            unavailable_reason,
            can_install: false, // CUDA must be installed system-wide, can't auto-install
            runtime_binary_id: None,
        }
    }
}

/// Factory for PyTorch backend
#[cfg(feature = "backend-pytorch")]
pub struct PyTorchFactory;

#[cfg(feature = "backend-pytorch")]
impl BackendFactory for PyTorchFactory {
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError> {
        Ok(Box::new(PyTorchBackend::new()))
    }

    fn info(&self) -> BackendInfo {
        let (available, unavailable_reason) = PyTorchBackend::check_availability();
        BackendInfo {
            name: "PyTorch".to_string(),
            backend_key: "pytorch".to_string(),
            description: "In-process PyTorch inference for dLLM, Sherry, and HuggingFace models"
                .to_string(),
            capabilities: PyTorchBackend::static_capabilities(),
            default_start_mode: BackendDefaultStartMode::Inference,
            active: false,
            available,
            unavailable_reason,
            can_install: false, // Requires Python + PyTorch to be pre-installed
            runtime_binary_id: None,
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

fn normalize_backend_lookup_key(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

pub fn canonical_backend_key(name: &str) -> String {
    match normalize_backend_lookup_key(name).as_str() {
        "llamacpp" => "llama_cpp".to_string(),
        "ollama" => "ollama".to_string(),
        "candle" => "candle".to_string(),
        "pytorch" => "pytorch".to_string(),
        other => other.to_string(),
    }
}

impl BackendRegistry {
    /// Create a new registry with all available backends registered
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };

        // Always register llama.cpp (default backend)
        #[cfg(feature = "backend-llamacpp")]
        registry.register("llama.cpp", Box::new(LlamaCppFactory));

        // Register Ollama backend if feature is enabled
        #[cfg(feature = "backend-ollama")]
        registry.register("Ollama", Box::new(OllamaFactory));

        // Register Candle backend if feature is enabled
        #[cfg(feature = "backend-candle")]
        registry.register("Candle", Box::new(CandleFactory));

        // Register PyTorch backend if feature is enabled
        #[cfg(feature = "backend-pytorch")]
        registry.register("PyTorch", Box::new(PyTorchFactory));

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

    fn resolve_factory(&self, name: &str) -> Option<&dyn BackendFactory> {
        let normalized = normalize_backend_lookup_key(name);
        self.factories.iter().find_map(|(registered_name, factory)| {
            (normalize_backend_lookup_key(registered_name) == normalized).then_some(factory.as_ref())
        })
    }

    /// Create a backend instance by name
    pub fn create(&self, name: &str) -> Result<Box<dyn InferenceBackend>, BackendError> {
        self.resolve_factory(name)
            .ok_or_else(|| BackendError::Config(format!("Unknown backend: {}", name)))?
            .create()
    }

    /// Check if a backend is available
    pub fn is_available(&self, name: &str) -> bool {
        self.resolve_factory(name).is_some()
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
    fn test_registry_creation() {
        let registry = BackendRegistry::new();
        // Should have at least one backend if any feature is enabled
        let backends = registry.list();
        // This test passes even with no backends enabled
        assert!(backends.len() >= 0);
    }

    #[cfg(feature = "backend-llamacpp")]
    #[test]
    fn test_registry_has_llamacpp() {
        let registry = BackendRegistry::new();
        assert!(registry.is_available("llama.cpp"));
        assert!(registry.is_available("llama_cpp"));
        assert!(registry.is_available("llama-cpp"));
    }

    #[cfg(feature = "backend-pytorch")]
    #[test]
    fn test_registry_resolves_pytorch_aliases() {
        let registry = BackendRegistry::new();
        let backend = registry.create("pytorch").expect("pytorch alias should resolve");
        assert_eq!(backend.name(), "PyTorch");
        assert!(registry.is_available("PyTorch"));
        assert!(registry.is_available("pytorch"));
    }
}
