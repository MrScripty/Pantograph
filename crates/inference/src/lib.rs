//! Multi-backend AI inference library
//!
//! This library provides a unified interface for different AI inference backends:
//! - **llama.cpp**: Local inference via GGUF models (default)
//! - **Ollama**: Integration with Ollama daemon
//! - **Candle**: In-process inference using Hugging Face Candle
//! - **PyTorch**: In-process PyO3 inference for dLLM, Sherry, and HuggingFace models
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
pub mod embedding_runtime;
pub mod gateway;
pub mod kv_cache;
pub mod managed_runtime;
pub mod process;
pub mod server;
pub mod types;

// Re-exports for convenience
pub use backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendFactory, BackendInfo, BackendRegistry,
    ChatChunk, EmbeddingResult, InferenceBackend,
};

#[cfg(feature = "backend-llamacpp")]
pub use backend::LlamaCppBackend;

#[cfg(feature = "backend-ollama")]
pub use backend::OllamaBackend;

#[cfg(feature = "backend-candle")]
pub use backend::CandleBackend;

#[cfg(feature = "backend-pytorch")]
pub use backend::PyTorchBackend;

pub use config::{DeviceConfig, EmbeddingMemoryMode};
pub use device::{list_llamacpp_devices, parse_llamacpp_device_listing, DeviceBackend};
pub use embedding_runtime::{DedicatedEmbeddingRuntimeManager, LlamaCppEmbeddingRuntime};
pub use gateway::{
    EmbeddingRuntimePreparation, EmbeddingStartRequest, GatewayError, InferenceGateway,
    InferenceStartRequest, SharedGateway,
};
pub use managed_runtime::{
    binary_capability, cancel_binary_download, check_binary_status, download_binary,
    list_binary_capabilities, list_managed_runtime_snapshots, load_managed_runtime_state,
    managed_runtime_dir, managed_runtime_snapshot, remove_binary, resolve_binary_command,
    save_managed_runtime_state, select_managed_runtime_version,
    set_default_managed_runtime_version, BinaryStatus, DownloadProgress,
    ManagedBinaryCapability, ManagedBinaryId, ManagedBinaryInstallState,
    ManagedRuntimeHistoryEventKind, ManagedRuntimeInstallHistoryEntry, ManagedRuntimeJobState,
    ManagedRuntimeJobStatus, ManagedRuntimePersistedRuntime, ManagedRuntimePersistedState,
    ManagedRuntimePersistedVersion, ManagedRuntimeReadinessState, ManagedRuntimeSelectionState,
    ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus, ResolvedCommand,
};
pub use process::{ProcessEvent, ProcessHandle, ProcessSpawner};
pub use server::{LlamaServer, ServerMode, SharedLlamaServer};
pub use types::{
    ChatMessage, ChatRequest, ContentPart, Delta, EncodedImage, ImageGenerationRequest,
    ImageGenerationResult, ImageUrlData, MaskedPrompt, PromptSegment, RerankRequest,
    RerankResponse, RerankResult, RuntimeLifecycleSnapshot, ServerModeInfo, StreamChoice,
    StreamChunk, StreamEvent,
};

#[cfg(feature = "std-process")]
pub use process::StdProcessSpawner;
