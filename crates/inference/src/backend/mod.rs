//! Pluggable inference backend abstraction
//!
//! This module provides a trait-based abstraction for different inference engines
//! (llama.cpp, Candle, PyTorch, external APIs). All backends implement the same
//! interface, allowing runtime switching between engines.

pub mod compatibility;
pub mod registry;

#[cfg(feature = "backend-llamacpp")]
pub mod llamacpp;

#[cfg(feature = "backend-candle")]
pub mod candle;

#[cfg(feature = "backend-pytorch")]
pub mod pytorch;

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};

use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::managed_runtime::ManagedBinaryId;
use crate::model_contracts::{
    BackendHintLabel, InferenceModality, InferenceTaskId, ModelArtifactKind, SupportTier,
    TaskModalitySignature,
};
use crate::process::ProcessSpawner;
use crate::types::{ImageGenerationRequest, ImageGenerationResult, RerankRequest, RerankResponse};

#[cfg(feature = "backend-llamacpp")]
pub use llamacpp::LlamaCppBackend;

#[cfg(feature = "backend-candle")]
pub use candle::CandleBackend;

#[cfg(feature = "backend-pytorch")]
pub use pytorch::PyTorchBackend;

pub use compatibility::{
    BackendCompatibilityIssue, BackendCompatibilityIssueKind, BackendCompatibilityOptions,
    BackendCompatibilityReport, BackendCompatibilityRequest, BackendCompatibilityStatus,
};
pub use registry::{canonical_backend_key, BackendFactory, BackendRegistry};

/// Error types for backend operations
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Backend not ready")]
    NotReady,

    #[error("Backend not running: {0}")]
    NotRunning(String),

    #[error("Startup failed: {0}")]
    StartupFailed(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Inference error: {0}")]
    Inference(String),

    #[error("Out of memory: {0}")]
    OutOfMemory(String),

    #[error("Managed binary error: {0}")]
    ManagedBinary(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Capabilities that a backend may or may not support
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendCapabilities {
    /// Supports vision/multimodal models (image + text)
    pub vision: bool,
    /// Supports image generation / diffusion requests
    pub image_generation: bool,
    /// Supports embedding generation
    pub embeddings: bool,
    /// Supports document reranking
    pub reranking: bool,
    /// Has GPU acceleration available
    pub gpu: bool,
    /// Allows manual GPU device selection
    pub device_selection: bool,
    /// Supports streaming token output
    pub streaming: bool,
    /// Supports tool/function calling
    pub tool_calling: bool,
    /// Supports attaching to an already-running external inference host.
    pub external_connection: bool,
    /// Structured task and modality facts that refine the legacy boolean flags.
    #[serde(default)]
    pub facts: BackendCapabilityFacts,
}

/// Structured backend capability facts aligned with canonical task semantics.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendCapabilityFacts {
    /// Canonical tasks this backend can execute without host-side policy
    /// inference.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tasks: Vec<BackendTaskCapability>,
    /// Whether preprocessing is handled inside the backend adapter, requires
    /// resolved package components, is unsupported, or is not needed.
    #[serde(default)]
    pub preprocessing: BackendComponentCapability,
    /// Whether postprocessing is handled inside the backend adapter, requires
    /// resolved package components, is unsupported, or is not needed.
    #[serde(default)]
    pub postprocessing: BackendComponentCapability,
    /// Static package source facts this backend can consume.
    #[serde(default)]
    pub model_sources: BackendModelSourceCapabilityFacts,
    /// Static support facts for cross-cutting execution features.
    #[serde(default)]
    pub features: BackendFeatureCapabilityFacts,
}

impl BackendCapabilityFacts {
    /// Build structured facts from canonical task capabilities.
    #[must_use]
    pub fn from_tasks(tasks: Vec<BackendTaskCapability>) -> Self {
        Self {
            tasks,
            preprocessing: BackendComponentCapability::Unknown,
            postprocessing: BackendComponentCapability::Unknown,
            model_sources: BackendModelSourceCapabilityFacts::default(),
            features: BackendFeatureCapabilityFacts::default(),
        }
    }

    /// Returns true when the backend declares support for the canonical task id
    /// at a non-roadmap, non-unsupported tier.
    #[must_use]
    pub fn supports_task(&self, task_id: InferenceTaskId) -> bool {
        self.tasks.iter().any(|task| {
            task.task_id == task_id
                && !matches!(
                    task.support_tier,
                    SupportTier::Roadmap | SupportTier::Unsupported | SupportTier::Unknown
                )
        })
    }
}

/// Static model package sources a backend adapter can load.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendModelSourceCapabilityFacts {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_kinds: Vec<ModelArtifactKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backend_hints: Vec<BackendHintLabel>,
    #[serde(default)]
    pub custom_code: BackendFeatureSupport,
}

/// Static backend support facts for execution features that cut across tasks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendFeatureCapabilityFacts {
    #[serde(default)]
    pub streaming: BackendFeatureSupport,
    #[serde(default)]
    pub device_selection: BackendFeatureSupport,
    #[serde(default)]
    pub external_connection: BackendFeatureSupport,
    #[serde(default)]
    pub kv_cache: BackendFeatureSupport,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendFeatureSupport {
    Supported,
    Unsupported,
    #[default]
    Unknown,
}

impl BackendFeatureSupport {
    /// Convert a legacy capability boolean into a structured support state.
    #[must_use]
    pub fn from_legacy_bool(supported: bool) -> Self {
        if supported {
            Self::Supported
        } else {
            Self::Unsupported
        }
    }
}

/// One canonical task supported by a backend adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendTaskCapability {
    pub task_id: InferenceTaskId,
    #[serde(default)]
    pub support_tier: SupportTier,
    pub modality_signature: TaskModalitySignature,
}

impl BackendTaskCapability {
    /// Construct a stable task capability.
    #[must_use]
    pub fn stable(
        task_id: InferenceTaskId,
        inputs: Vec<InferenceModality>,
        outputs: Vec<InferenceModality>,
    ) -> Self {
        Self {
            task_id,
            support_tier: SupportTier::Stable,
            modality_signature: TaskModalitySignature::new(inputs, outputs),
        }
    }
}

/// Backend-local component handling for pre/post process facts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendComponentCapability {
    /// Capability has not yet been audited for this backend.
    #[default]
    Unknown,
    /// The task does not need this lifecycle stage.
    NotRequired,
    /// The backend adapter owns this lifecycle stage.
    BackendManaged,
    /// The backend needs package components such as tokenizer, processor, or
    /// chat template facts.
    RequiresPackageComponent,
    /// The backend cannot perform this lifecycle stage.
    Unsupported,
}

impl BackendCapabilities {
    /// Returns true when structured facts declare support for a canonical task.
    ///
    /// This helper intentionally does not consult scheduler/runtime state.
    #[must_use]
    pub fn supports_task(&self, task_id: InferenceTaskId) -> bool {
        self.facts.supports_task(task_id)
    }
}

#[cfg(test)]
mod capability_tests {
    use futures_util::stream;
    use serde_json::json;

    use super::*;

    struct UnsupportedBackend;

    #[async_trait]
    impl InferenceBackend for UnsupportedBackend {
        fn name(&self) -> &'static str {
            "unsupported"
        }

        fn description(&self) -> &'static str {
            "unsupported backend"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            Ok(BackendStartOutcome::default())
        }

        fn stop(&mut self) {}

        fn is_ready(&self) -> bool {
            true
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn base_url(&self) -> Option<String> {
            None
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
        {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }
    }

    #[test]
    fn backend_capabilities_deserialize_without_structured_facts() {
        let capabilities = serde_json::from_value::<BackendCapabilities>(json!({
            "vision": true,
            "image_generation": false,
            "embeddings": true,
            "reranking": false,
            "gpu": true,
            "device_selection": true,
            "streaming": true,
            "tool_calling": false,
            "external_connection": false
        }))
        .expect("legacy capability payload should deserialize");

        assert!(capabilities.vision);
        assert!(capabilities.facts.tasks.is_empty());
        assert_eq!(
            capabilities.facts.features.streaming,
            BackendFeatureSupport::Unknown
        );
        assert!(!capabilities.supports_task(InferenceTaskId::Embedding));
    }

    #[test]
    fn backend_capability_facts_report_supported_tasks() {
        let capabilities = BackendCapabilities {
            facts: BackendCapabilityFacts::from_tasks(vec![BackendTaskCapability::stable(
                InferenceTaskId::Embedding,
                vec![InferenceModality::Text],
                vec![InferenceModality::Embedding],
            )]),
            ..BackendCapabilities::default()
        };

        assert!(capabilities.supports_task(InferenceTaskId::Embedding));
        assert!(!capabilities.supports_task(InferenceTaskId::Rerank));
    }

    #[test]
    fn backend_feature_support_maps_legacy_booleans() {
        assert_eq!(
            BackendFeatureSupport::from_legacy_bool(true),
            BackendFeatureSupport::Supported
        );
        assert_eq!(
            BackendFeatureSupport::from_legacy_bool(false),
            BackendFeatureSupport::Unsupported
        );
    }

    #[tokio::test]
    async fn default_image_generation_returns_explicit_unsupported_error() {
        let error = UnsupportedBackend
            .generate_image(ImageGenerationRequest {
                model: "model".to_string(),
                prompt: "prompt".to_string(),
                negative_prompt: None,
                width: None,
                height: None,
                num_inference_steps: None,
                guidance_scale: None,
                seed: None,
                scheduler: None,
                num_images_per_prompt: None,
                init_image: None,
                mask_image: None,
                strength: None,
                extra_options: serde_json::Value::Null,
            })
            .await
            .expect_err("default image generation should be unsupported");

        assert!(
            matches!(error, BackendError::Inference(message) if message.contains("Image generation not supported"))
        );
    }

    #[tokio::test]
    async fn default_kv_cache_fingerprint_returns_explicit_unsupported_error() {
        let error = UnsupportedBackend
            .kv_cache_runtime_fingerprint(None)
            .await
            .expect_err("default kv-cache fingerprint should be unsupported");

        assert!(
            matches!(error, BackendError::Inference(message) if message.contains("KV cache runtime fingerprint not supported"))
        );
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendDefaultStartMode {
    Inference,
    Embedding,
}

/// Backend information for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    /// Backend identifier (e.g., "llama.cpp", "Candle", "PyTorch")
    pub name: String,
    /// Stable backend key for contracts and selection state.
    pub backend_key: String,
    /// Human-readable description
    pub description: String,
    /// Backend capabilities
    pub capabilities: BackendCapabilities,
    /// Backend-owned recommended mode to start when the host selects this backend.
    pub default_start_mode: BackendDefaultStartMode,
    /// Whether this backend is currently active
    pub active: bool,
    /// Whether this backend is available (dependencies met)
    pub available: bool,
    /// Reason if unavailable
    pub unavailable_reason: Option<String>,
    /// Whether this backend can be auto-installed (binaries can be downloaded)
    pub can_install: bool,
    /// Managed runtime backing this backend, when applicable.
    #[serde(default)]
    pub runtime_binary_id: Option<ManagedBinaryId>,
}

/// Configuration for starting a backend
#[derive(Debug, Clone, Default)]
pub struct BackendConfig {
    /// External OpenAI-compatible base URL (for remote or already-running hosts)
    pub external_url: Option<String>,
    /// Optional host-selected port for managed HTTP sidecars.
    ///
    /// This remains backend-owned transport config rather than host-local
    /// recovery policy so restart flows can preserve the requested port
    /// through the normal backend start contract.
    pub port_override: Option<u16>,
    /// Model file path (for llama.cpp GGUF files)
    pub model_path: Option<std::path::PathBuf>,
    /// Vision projection file path (for llama.cpp mmproj)
    pub mmproj_path: Option<std::path::PathBuf>,
    /// Model name for external or compatibility backends that identify models
    /// by name instead of local path.
    pub model_name: Option<String>,
    /// HuggingFace model ID (for Candle)
    pub model_id: Option<String>,
    /// Device configuration
    pub device: Option<String>,
    /// Number of GPU layers (-1 for all)
    pub gpu_layers: Option<i32>,
    /// Context size
    pub context_size: Option<u32>,
    /// Embedding mode
    pub embedding_mode: bool,
    /// Reranking mode
    pub reranking_mode: bool,
    /// Model type hint for PyTorch backend (dllm, sherry, text-generation).
    /// If None, auto-detected from config.json.
    pub model_type: Option<String>,
}

/// Backend-owned outcome for a successful runtime start request.
#[derive(Debug, Clone, Default)]
pub struct BackendStartOutcome {
    /// Whether the backend attached to an already-running runtime instead of
    /// launching a fresh one.
    pub runtime_reused: Option<bool>,
    /// Structured reason describing the lifecycle decision taken by the backend.
    pub lifecycle_decision_reason: Option<String>,
}

/// A streaming chunk from chat completion
#[derive(Debug, Clone, Serialize)]
pub struct ChatChunk {
    /// Text content of this chunk
    pub content: Option<String>,
    /// Whether this is the final chunk
    pub done: bool,
}

/// Embedding result
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingResult {
    /// The embedding vector
    pub vector: Vec<f32>,
    /// Number of tokens in the input
    pub token_count: usize,
}

/// Re-export diffusion request/result types from the shared `types` module so
/// backend consumers can reach them from the backend facade.
pub type ImageRequest = ImageGenerationRequest;
pub type ImageResult = ImageGenerationResult;

/// The core trait that all inference backends must implement.
///
/// Backends can be HTTP-based (llama.cpp, External) or in-process (Candle).
/// All use a common interface that application code can call without knowing
/// which backend is active.
#[async_trait]
pub trait InferenceBackend: Send + Sync {
    // ─── IDENTITY ───────────────────────────────────────────────────

    /// Human-readable name for UI display
    fn name(&self) -> &'static str;

    /// Description of this backend
    fn description(&self) -> &'static str;

    /// What this backend supports
    fn capabilities(&self) -> BackendCapabilities;

    // ─── LIFECYCLE ──────────────────────────────────────────────────

    /// Initialize and start the backend with given configuration
    ///
    /// # Arguments
    /// * `config` - Backend configuration (model paths, device settings, etc.)
    /// * `spawner` - Process spawner for launching sidecar processes
    async fn start(
        &mut self,
        config: &BackendConfig,
        spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError>;

    /// Stop the backend and cleanup resources
    fn stop(&mut self);

    /// Is the backend ready to accept requests?
    fn is_ready(&self) -> bool;

    /// Health check - verify the backend is responding
    async fn health_check(&self) -> bool;

    /// Get the base URL for this backend (if HTTP-based)
    /// Returns None for in-process backends like Candle
    fn base_url(&self) -> Option<String>;

    // ─── INFERENCE ──────────────────────────────────────────────────

    /// Stream chat completion responses
    ///
    /// Takes a JSON-serialized OpenAI-compatible chat completion request
    /// and returns a stream of response chunks.
    async fn chat_completion_stream(
        &self,
        request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>;

    /// Generate embeddings for the given texts
    async fn embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError>;

    /// Rank candidate documents against a query.
    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse, BackendError>;

    /// Generate one or more images from a diffusion-capable backend.
    async fn generate_image(
        &self,
        _request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, BackendError> {
        Err(BackendError::Inference(
            "Image generation not supported by this backend".to_string(),
        ))
    }

    /// Describe the active runtime semantics that govern whether one KV artifact
    /// may be reused by this backend.
    async fn kv_cache_runtime_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<KvCacheRuntimeFingerprint, BackendError> {
        Err(BackendError::Inference(
            "KV cache runtime fingerprint not supported by this backend".to_string(),
        ))
    }

    /// Describe the active model configuration for KV-compatibility checks.
    async fn kv_cache_model_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<ModelFingerprint, BackendError> {
        Err(BackendError::Inference(
            "KV cache model fingerprint not supported by this backend".to_string(),
        ))
    }

    /// Persist the active runtime slot state into a backend-owned file.
    async fn save_kv_cache_slot(&self, _slot_id: u32, _path: &Path) -> Result<(), BackendError> {
        Err(BackendError::Inference(
            "KV cache slot save not supported by this backend".to_string(),
        ))
    }

    /// Restore a backend-owned file into a live runtime slot.
    async fn restore_kv_cache_slot(&self, _slot_id: u32, _path: &Path) -> Result<(), BackendError> {
        Err(BackendError::Inference(
            "KV cache slot restore not supported by this backend".to_string(),
        ))
    }

    /// Clear the active runtime slot state after a restore, failure, or reset.
    async fn clear_kv_cache_slot(&self, _slot_id: u32) -> Result<(), BackendError> {
        Err(BackendError::Inference(
            "KV cache slot clear not supported by this backend".to_string(),
        ))
    }

    /// Truncate a backend-owned KV artifact to the requested token position.
    async fn truncate_kv_cache_data(
        &self,
        _data: &[u8],
        _token_position: usize,
        _active_config: Option<&BackendConfig>,
    ) -> Result<Vec<u8>, BackendError> {
        Err(BackendError::Inference(
            "KV cache truncation not supported by this backend".to_string(),
        ))
    }
}
