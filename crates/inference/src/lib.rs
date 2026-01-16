//! Multi-backend AI inference library
//!
//! This library provides a unified interface for different AI inference backends:
//! - **llama.cpp**: Local inference via GGUF models (default)
//! - **Ollama**: Integration with Ollama daemon
//! - **Candle**: In-process inference using Hugging Face Candle
//!
//! # Example
//!
//! ```rust,ignore
//! use inference::{InferenceGateway, BackendConfig, ProcessSpawner};
//! use std::sync::Arc;
//!
//! // Create a gateway with your process spawner implementation
//! let gateway = InferenceGateway::new();
//!
//! // Configure and start a backend
//! let config = BackendConfig {
//!     model_path: Some("/path/to/model.gguf".into()),
//!     ..Default::default()
//! };
//!
//! gateway.start(&config, spawner).await?;
//! ```

pub mod backend;
pub mod config;
pub mod constants;
pub mod device;
pub mod gateway;
pub mod process;
pub mod server;
pub mod types;

// Re-exports for convenience
pub use backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendFactory, BackendInfo,
    BackendRegistry, ChatChunk, EmbeddingResult, InferenceBackend,
};

#[cfg(feature = "backend-llamacpp")]
pub use backend::LlamaCppBackend;

#[cfg(feature = "backend-ollama")]
pub use backend::OllamaBackend;

#[cfg(feature = "backend-candle")]
pub use backend::CandleBackend;

pub use config::{DeviceConfig, EmbeddingMemoryMode};
pub use device::DeviceBackend;
pub use gateway::{GatewayError, InferenceGateway, SharedGateway};
pub use process::{ProcessEvent, ProcessHandle, ProcessSpawner};
pub use server::{LlamaServer, ServerMode, SharedLlamaServer};
pub use types::LLMStatus;

#[cfg(feature = "std-process")]
pub use process::StdProcessSpawner;
