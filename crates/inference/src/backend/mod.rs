//! Pluggable inference backend abstraction
//!
//! This module provides a trait-based abstraction for different inference engines
//! (llama.cpp, Candle, PyTorch, external APIs). All backends implement the same
//! interface, allowing runtime switching between engines.

pub mod compatibility;
pub mod registry;
mod startup_device;

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

use crate::device_contracts::InferenceDevicePolicy;
use crate::device_contracts::RuntimeVariantCapability;
#[cfg(any(
    test,
    feature = "backend-llamacpp",
    feature = "backend-candle",
    feature = "backend-pytorch",
))]
use crate::device_contracts::{
    BackendId, DeviceResolutionDiagnostic, DeviceResolutionDiagnosticCode,
    DeviceResolutionDiagnosticSeverity, InferenceDeviceClass, RuntimeVariantId,
};
use crate::execution_telemetry::BackendExecutionContext;
use crate::image_generation_planner::ImageGenerationExecutionPlan;
use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::managed_runtime::ManagedBinaryId;
use crate::model_contracts::{
    BackendHintLabel, InferenceLifecyclePhase, InferenceModality, InferenceTaskId,
    ModelArtifactKind, SupportTier, TaskModalitySignature,
};
use crate::process::ProcessSpawner;
use crate::runtime_load::LlamaCppActiveRuntimeDescriptor;
use crate::types::{
    AudioTranscriptionRequest, AudioTranscriptionResult, ImageGenerationRequest,
    ImageGenerationResult, InferenceUsage, RerankRequest, RerankResponse,
};
use crate::{config::DeviceConfig, constants::defaults, device::DeviceBackend};

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
pub use startup_device::{BackendStartupDeviceIntent, BackendStartupDeviceIntentError};

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
    /// Backend-owned runtime variant facts exposed to scheduler/admission
    /// without ranking policy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_variants: Vec<RuntimeVariantCapability>,
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
            runtime_variants: Vec::new(),
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

    /// Derive request lifecycle facts from static backend capability facts.
    ///
    /// These facts describe observable or adapter-owned cancellation/cleanup
    /// behavior. They are not scheduler policy and do not decide whether a
    /// runtime should be selected.
    #[must_use]
    pub fn request_lifecycle_facts(&self) -> BackendRequestLifecycleFacts {
        BackendRequestLifecycleFacts {
            phases: vec![
                BackendRequestLifecyclePhaseFacts {
                    phase: InferenceLifecyclePhase::ModelPackageResolution,
                    component: BackendComponentCapability::NotRequired,
                    cancellation: BackendRequestCancellationSemantics::NotApplicable,
                    cleanup: BackendRequestCleanupSemantics::NotRequired,
                },
                BackendRequestLifecyclePhaseFacts {
                    phase: InferenceLifecyclePhase::TaskValidation,
                    component: BackendComponentCapability::NotRequired,
                    cancellation: BackendRequestCancellationSemantics::NotApplicable,
                    cleanup: BackendRequestCleanupSemantics::NotRequired,
                },
                BackendRequestLifecyclePhaseFacts::from_component_phase(
                    InferenceLifecyclePhase::Preprocessing,
                    self.preprocessing,
                ),
                BackendRequestLifecyclePhaseFacts {
                    phase: InferenceLifecyclePhase::BackendExecution,
                    component: BackendComponentCapability::BackendManaged,
                    cancellation: if self.features.streaming == BackendFeatureSupport::Supported {
                        BackendRequestCancellationSemantics::DropConsumer
                    } else {
                        BackendRequestCancellationSemantics::AdapterManaged
                    },
                    cleanup: if self.features.streaming == BackendFeatureSupport::Supported {
                        BackendRequestCleanupSemantics::DropStream
                    } else {
                        BackendRequestCleanupSemantics::AdapterManaged
                    },
                },
                BackendRequestLifecyclePhaseFacts::from_component_phase(
                    InferenceLifecyclePhase::Postprocessing,
                    self.postprocessing,
                ),
                BackendRequestLifecyclePhaseFacts {
                    phase: InferenceLifecyclePhase::ResultProjection,
                    component: BackendComponentCapability::NotRequired,
                    cancellation: BackendRequestCancellationSemantics::NotApplicable,
                    cleanup: BackendRequestCleanupSemantics::NotRequired,
                },
            ],
            kv_cache_publication_cleanup: if self.features.kv_cache
                == BackendFeatureSupport::Supported
            {
                BackendRequestCleanupSemantics::RollbackPublication
            } else {
                BackendRequestCleanupSemantics::NotApplicable
            },
        }
    }
}

#[cfg(any(
    test,
    feature = "backend-llamacpp",
    feature = "backend-candle",
    feature = "backend-pytorch",
))]
pub(crate) fn available_runtime_variant_capability(
    backend_id: &'static str,
    runtime_variant_id: &'static str,
    device_class: InferenceDeviceClass,
) -> RuntimeVariantCapability {
    let _ = backend_id_from_static(backend_id);
    RuntimeVariantCapability {
        runtime_variant_id: runtime_variant_id_from_static(runtime_variant_id),
        device_class,
        available: true,
        diagnostics: Vec::new(),
    }
}

#[cfg(any(
    test,
    feature = "backend-llamacpp",
    feature = "backend-candle",
    feature = "backend-pytorch",
))]
pub(crate) fn unavailable_runtime_variant_capability(
    backend_id: &'static str,
    runtime_variant_id: &'static str,
    device_class: InferenceDeviceClass,
    code: DeviceResolutionDiagnosticCode,
    message: &'static str,
) -> RuntimeVariantCapability {
    let backend_id = backend_id_from_static(backend_id);
    let runtime_variant_id = runtime_variant_id_from_static(runtime_variant_id);
    RuntimeVariantCapability {
        runtime_variant_id: runtime_variant_id.clone(),
        device_class,
        available: false,
        diagnostics: vec![DeviceResolutionDiagnostic {
            code,
            severity: DeviceResolutionDiagnosticSeverity::Error,
            message: message.to_string(),
            device_class: Some(device_class),
            device_id: None,
            runtime_variant_id: Some(runtime_variant_id),
            backend_id: Some(backend_id),
        }],
    }
}

#[cfg(any(
    test,
    feature = "backend-llamacpp",
    feature = "backend-candle",
    feature = "backend-pytorch",
))]
fn backend_id_from_static(value: &'static str) -> BackendId {
    BackendId::parse(value).expect("static backend id must satisfy device contract validation")
}

#[cfg(any(
    test,
    feature = "backend-llamacpp",
    feature = "backend-candle",
    feature = "backend-pytorch",
))]
fn runtime_variant_id_from_static(value: &'static str) -> RuntimeVariantId {
    RuntimeVariantId::parse(value)
        .expect("static runtime variant id must satisfy device contract validation")
}

/// Static request lifecycle semantics derived from backend capability facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendRequestLifecycleFacts {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<BackendRequestLifecyclePhaseFacts>,
    #[serde(default)]
    pub kv_cache_publication_cleanup: BackendRequestCleanupSemantics,
}

/// Cancellation and cleanup semantics for one inference lifecycle phase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendRequestLifecyclePhaseFacts {
    pub phase: InferenceLifecyclePhase,
    pub component: BackendComponentCapability,
    pub cancellation: BackendRequestCancellationSemantics,
    pub cleanup: BackendRequestCleanupSemantics,
}

impl BackendRequestLifecyclePhaseFacts {
    fn from_component_phase(
        phase: InferenceLifecyclePhase,
        component: BackendComponentCapability,
    ) -> Self {
        let (cancellation, cleanup) = match component {
            BackendComponentCapability::NotRequired => (
                BackendRequestCancellationSemantics::NotApplicable,
                BackendRequestCleanupSemantics::NotRequired,
            ),
            BackendComponentCapability::BackendManaged
            | BackendComponentCapability::RequiresPackageComponent => (
                BackendRequestCancellationSemantics::AdapterManaged,
                BackendRequestCleanupSemantics::AdapterManaged,
            ),
            BackendComponentCapability::Unsupported => (
                BackendRequestCancellationSemantics::NotSupported,
                BackendRequestCleanupSemantics::NotApplicable,
            ),
            BackendComponentCapability::Unknown => (
                BackendRequestCancellationSemantics::Unknown,
                BackendRequestCleanupSemantics::Unknown,
            ),
        };

        Self {
            phase,
            component,
            cancellation,
            cleanup,
        }
    }
}

/// How cancellation is handled for a request lifecycle phase.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendRequestCancellationSemantics {
    #[default]
    Unknown,
    NotApplicable,
    NotSupported,
    AdapterManaged,
    DropConsumer,
}

/// How cleanup is handled for a request lifecycle phase or publication step.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendRequestCleanupSemantics {
    #[default]
    Unknown,
    NotApplicable,
    NotRequired,
    AdapterManaged,
    DropStream,
    RollbackPublication,
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
        assert!(capabilities.facts.runtime_variants.is_empty());
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
    fn backend_capability_facts_preserve_runtime_variant_facts() {
        let facts = BackendCapabilityFacts {
            runtime_variants: vec![
                available_runtime_variant_capability(
                    "pytorch",
                    "pytorch.cpu",
                    InferenceDeviceClass::Cpu,
                ),
                unavailable_runtime_variant_capability(
                    "pytorch",
                    "pytorch.cuda",
                    InferenceDeviceClass::Cuda,
                    DeviceResolutionDiagnosticCode::MissingRuntimeVariant,
                    "PyTorch CUDA runtime variant readiness is not reported",
                ),
            ],
            ..BackendCapabilityFacts::default()
        };

        let encoded = serde_json::to_value(&facts).expect("runtime variant facts encode");
        assert_eq!(
            encoded["runtime_variants"][0]["runtime_variant_id"],
            "pytorch.cpu"
        );
        assert_eq!(encoded["runtime_variants"][0]["device_class"], "cpu");
        assert_eq!(encoded["runtime_variants"][0]["available"], true);
        assert_eq!(
            encoded["runtime_variants"][1]["diagnostics"][0]["code"],
            "missing_runtime_variant"
        );

        let decoded: BackendCapabilityFacts =
            serde_json::from_value(encoded).expect("runtime variant facts decode");
        assert_eq!(decoded.runtime_variants.len(), 2);
        assert_eq!(
            decoded.runtime_variants[1].diagnostics[0]
                .backend_id
                .as_ref()
                .map(BackendId::as_str),
            Some("pytorch")
        );
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

    #[test]
    fn request_lifecycle_facts_derive_stream_and_kv_cleanup_semantics() {
        let facts = BackendCapabilityFacts {
            preprocessing: BackendComponentCapability::RequiresPackageComponent,
            postprocessing: BackendComponentCapability::BackendManaged,
            features: BackendFeatureCapabilityFacts {
                streaming: BackendFeatureSupport::Supported,
                kv_cache: BackendFeatureSupport::Supported,
                ..BackendFeatureCapabilityFacts::default()
            },
            ..BackendCapabilityFacts::default()
        };

        let lifecycle = facts.request_lifecycle_facts();
        let backend_execution = lifecycle
            .phases
            .iter()
            .find(|phase| phase.phase == InferenceLifecyclePhase::BackendExecution)
            .expect("backend execution phase");
        let preprocessing = lifecycle
            .phases
            .iter()
            .find(|phase| phase.phase == InferenceLifecyclePhase::Preprocessing)
            .expect("preprocessing phase");

        assert_eq!(
            preprocessing.cancellation,
            BackendRequestCancellationSemantics::AdapterManaged
        );
        assert_eq!(
            backend_execution.cancellation,
            BackendRequestCancellationSemantics::DropConsumer
        );
        assert_eq!(
            backend_execution.cleanup,
            BackendRequestCleanupSemantics::DropStream
        );
        assert_eq!(
            lifecycle.kv_cache_publication_cleanup,
            BackendRequestCleanupSemantics::RollbackPublication
        );
    }

    #[test]
    fn request_lifecycle_facts_mark_unsupported_component_semantics() {
        let lifecycle = BackendCapabilityFacts {
            preprocessing: BackendComponentCapability::Unsupported,
            postprocessing: BackendComponentCapability::NotRequired,
            features: BackendFeatureCapabilityFacts {
                streaming: BackendFeatureSupport::Unsupported,
                kv_cache: BackendFeatureSupport::Unsupported,
                ..BackendFeatureCapabilityFacts::default()
            },
            ..BackendCapabilityFacts::default()
        }
        .request_lifecycle_facts();

        let preprocessing = lifecycle
            .phases
            .iter()
            .find(|phase| phase.phase == InferenceLifecyclePhase::Preprocessing)
            .expect("preprocessing phase");
        let postprocessing = lifecycle
            .phases
            .iter()
            .find(|phase| phase.phase == InferenceLifecyclePhase::Postprocessing)
            .expect("postprocessing phase");
        let backend_execution = lifecycle
            .phases
            .iter()
            .find(|phase| phase.phase == InferenceLifecyclePhase::BackendExecution)
            .expect("backend execution phase");

        assert_eq!(
            preprocessing.cancellation,
            BackendRequestCancellationSemantics::NotSupported
        );
        assert_eq!(
            postprocessing.cleanup,
            BackendRequestCleanupSemantics::NotRequired
        );
        assert_eq!(
            backend_execution.cancellation,
            BackendRequestCancellationSemantics::AdapterManaged
        );
        assert_eq!(
            lifecycle.kv_cache_publication_cleanup,
            BackendRequestCleanupSemantics::NotApplicable
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
                denoising_scheduler: None,
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
#[derive(Debug, Clone)]
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
    /// Typed startup device intent.
    pub device: Option<BackendStartupDeviceIntent>,
    /// Number of GPU layers (-1 for all)
    pub gpu_layers: Option<i32>,
    /// Context size
    pub context_size: Option<u32>,
    /// CPU threads for llama.cpp token generation.
    pub cpu_threads: Option<u32>,
    /// Logical llama.cpp batch size.
    pub batch_size: Option<u32>,
    /// Physical llama.cpp micro-batch size.
    pub ubatch_size: Option<u32>,
    /// Embedding mode
    pub embedding_mode: bool,
    /// Reranking mode
    pub reranking_mode: bool,
    /// Model type hint for PyTorch backend (dllm, sherry, text-generation).
    /// If None, auto-detected from config.json.
    pub model_type: Option<String>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            external_url: None,
            port_override: None,
            model_path: None,
            mmproj_path: None,
            model_name: None,
            model_id: None,
            device: Some(BackendStartupDeviceIntent::scheduler_policy(
                InferenceDevicePolicy::Auto,
            )),
            gpu_layers: None,
            context_size: None,
            cpu_threads: None,
            batch_size: None,
            ubatch_size: None,
            embedding_mode: false,
            reranking_mode: false,
            model_type: None,
        }
    }
}

/// Backend-owned effective llama.cpp runtime settings.
///
/// This is the normalization boundary for settings that affect llama.cpp
/// process startup. Hosts may transport user preferences, but the inference
/// crate owns the effective execution values consumed by runtime startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppRuntimeSettings {
    /// Effective llama.cpp device selector.
    pub device: DeviceBackend,
    /// Effective number of GPU layers (`-1` means all layers).
    pub gpu_layers: i32,
    /// Effective context size used for llama-server `-c`.
    pub context_size: u32,
    /// Optional CPU thread count for llama.cpp `-t`.
    pub cpu_threads: Option<u32>,
    /// Optional logical batch size for llama.cpp `-b`.
    pub batch_size: Option<u32>,
    /// Optional physical micro-batch size for llama.cpp `-ub`.
    pub ubatch_size: Option<u32>,
}

impl LlamaCppRuntimeSettings {
    /// Normalize and validate a backend start config into effective settings.
    pub fn try_from_backend_config(config: &BackendConfig) -> Result<Self, BackendError> {
        validate_optional_positive_u32(config.context_size, "context_size")?;
        validate_optional_positive_u32(config.cpu_threads, "cpu_threads")?;
        validate_optional_positive_u32(config.batch_size, "batch_size")?;
        validate_optional_positive_u32(config.ubatch_size, "ubatch_size")?;
        let device = validate_llamacpp_device(config.device.as_ref())?;
        Ok(Self {
            device,
            gpu_layers: config.gpu_layers.unwrap_or(defaults::GPU_LAYERS),
            context_size: config.context_size.unwrap_or(defaults::CONTEXT_SIZE),
            cpu_threads: config.cpu_threads,
            batch_size: config.batch_size,
            ubatch_size: config.ubatch_size,
        })
    }

    /// Project effective settings into the existing sidecar device DTO.
    #[must_use]
    pub fn device_config(&self) -> DeviceConfig {
        DeviceConfig {
            device: self.device.clone(),
            gpu_layers: self.gpu_layers,
        }
    }
}

fn validate_llamacpp_device(
    device: Option<&BackendStartupDeviceIntent>,
) -> Result<DeviceBackend, BackendError> {
    let Some(device) = device else {
        return Err(BackendError::Config(
            "llama.cpp device setting is required; use explicit auto policy when scheduler-owned selection is intended"
                .to_string(),
        ));
    };
    match device {
        BackendStartupDeviceIntent::LlamaCppSelector(selector) => Ok(selector.clone()),
        BackendStartupDeviceIntent::SchedulerPolicy(InferenceDevicePolicy::Auto) => {
            Ok(DeviceBackend::Auto)
        }
        BackendStartupDeviceIntent::SchedulerPolicy(InferenceDevicePolicy::Explicit { .. }) => {
            Err(BackendError::Config(
                "llama.cpp startup requires a resolved backend-local device selector, not an unresolved explicit scheduler policy"
                    .to_string(),
            ))
        }
        BackendStartupDeviceIntent::CanonicalDevice(device_id) => Err(BackendError::Config(
            format!(
                "llama.cpp startup does not accept canonical device id '{}'",
                device_id.as_str()
            ),
        )),
    }
}

fn validate_optional_positive_u32(
    value: Option<u32>,
    field_name: &str,
) -> Result<(), BackendError> {
    if value == Some(0) {
        return Err(BackendError::Config(format!(
            "llama.cpp runtime setting '{field_name}' must be greater than zero when provided"
        )));
    }
    Ok(())
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
    /// Optional bounded usage counts, usually emitted on the terminal chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<InferenceUsage>,
    /// Optional backend-local KV cache handle id, usually emitted on the
    /// terminal chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_handle_id: Option<String>,
}

/// Embedding result
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingResult {
    /// The embedding vector
    pub vector: Vec<f32>,
    /// Number of tokens in the input
    pub token_count: usize,
}

/// Extract per-item embedding token usage only when attribution is unambiguous.
///
/// OpenAI-compatible embedding responses report usage at response scope rather
/// than per embedding item. For multi-input batches we keep `token_count = 0`
/// instead of distributing counts heuristically.
#[cfg(any(feature = "backend-llamacpp", feature = "backend-candle", test))]
pub(crate) fn openai_embedding_token_count_for_single_result(
    response_json: &serde_json::Value,
    item_count: usize,
) -> usize {
    if item_count != 1 {
        return 0;
    }
    response_json
        .get("usage")
        .and_then(|usage| {
            usage
                .get("prompt_tokens")
                .or_else(|| usage.get("total_tokens"))
        })
        .and_then(|value| value.as_u64())
        .and_then(|count| u32::try_from(count).ok())
        .map(|count| count as usize)
        .unwrap_or(0)
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

    /// Structured llama.cpp runtime identity when this backend owns a ready
    /// managed llama.cpp sidecar.
    fn active_llamacpp_runtime_descriptor(&self) -> Option<LlamaCppActiveRuntimeDescriptor> {
        None
    }

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

    /// Generate one or more images from a canonical planned execution context.
    async fn generate_image_from_plan(
        &self,
        _plan: ImageGenerationExecutionPlan,
        _context: BackendExecutionContext,
    ) -> Result<ImageGenerationResult, BackendError> {
        Err(BackendError::Inference(
            "Planned image generation not supported by this backend".to_string(),
        ))
    }

    /// Transcribe audio through a speech-to-text capable backend.
    async fn transcribe_audio(
        &self,
        _request: AudioTranscriptionRequest,
    ) -> Result<AudioTranscriptionResult, BackendError> {
        Err(BackendError::Inference(
            "Audio transcription not supported by this backend".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_config_default_uses_explicit_auto_device() {
        assert!(matches!(
            BackendConfig::default().device,
            Some(BackendStartupDeviceIntent::SchedulerPolicy(
                InferenceDevicePolicy::Auto
            ))
        ));
    }

    #[test]
    fn llamacpp_runtime_settings_normalize_backend_config_defaults() {
        let settings = LlamaCppRuntimeSettings::try_from_backend_config(&BackendConfig {
            gpu_layers: None,
            context_size: None,
            ..BackendConfig::default()
        })
        .expect("default backend config should be valid explicit auto");

        assert_eq!(settings.device, DeviceBackend::Auto);
        assert_eq!(settings.gpu_layers, defaults::GPU_LAYERS);
        assert_eq!(settings.context_size, defaults::CONTEXT_SIZE);
        assert_eq!(settings.cpu_threads, None);
        assert_eq!(settings.batch_size, None);
        assert_eq!(settings.ubatch_size, None);
        assert_eq!(
            settings.device_config(),
            DeviceConfig {
                device: DeviceBackend::Auto,
                gpu_layers: defaults::GPU_LAYERS,
            }
        );
    }

    #[test]
    fn llamacpp_runtime_settings_preserve_explicit_backend_config() {
        let settings = LlamaCppRuntimeSettings::try_from_backend_config(&BackendConfig {
            device: Some(
                BackendStartupDeviceIntent::llama_cpp_selector("Vulkan0")
                    .expect("valid llama.cpp selector"),
            ),
            gpu_layers: Some(42),
            context_size: Some(8192),
            cpu_threads: Some(8),
            batch_size: Some(512),
            ubatch_size: Some(128),
            ..BackendConfig::default()
        })
        .expect("explicit backend config should be valid");

        assert_eq!(
            settings,
            LlamaCppRuntimeSettings {
                device: DeviceBackend::Vulkan(0),
                gpu_layers: 42,
                context_size: 8192,
                cpu_threads: Some(8),
                batch_size: Some(512),
                ubatch_size: Some(128),
            }
        );
    }

    #[test]
    fn gpu_layers_remain_llamacpp_runtime_setting_not_device_policy() {
        let settings = LlamaCppRuntimeSettings::try_from_backend_config(&BackendConfig {
            device: Some(
                BackendStartupDeviceIntent::llama_cpp_selector("CUDA0")
                    .expect("valid llama.cpp selector"),
            ),
            gpu_layers: Some(42),
            ..BackendConfig::default()
        })
        .expect("explicit backend config should be valid");
        assert_eq!(settings.device_config().gpu_layers, 42);

        let policy = crate::device_contracts::InferenceDevicePolicy::Explicit {
            device_class: InferenceDeviceClass::Cuda,
            device_id: Some(
                crate::device_contracts::InferenceDeviceId::parse("cuda:0")
                    .expect("valid device id"),
            ),
        };
        let encoded = serde_json::to_value(policy).expect("device policy should serialize");
        for backend_local_field in ["gpu_layers", "offload", "hybrid", "split"] {
            assert!(
                encoded.get(backend_local_field).is_none(),
                "canonical device policy must not expose backend-local {backend_local_field}"
            );
        }
    }

    #[test]
    fn llamacpp_runtime_settings_reject_zero_sized_performance_knobs() {
        for (field_name, config) in [
            (
                "context_size",
                BackendConfig {
                    context_size: Some(0),
                    ..BackendConfig::default()
                },
            ),
            (
                "cpu_threads",
                BackendConfig {
                    cpu_threads: Some(0),
                    ..BackendConfig::default()
                },
            ),
            (
                "batch_size",
                BackendConfig {
                    batch_size: Some(0),
                    ..BackendConfig::default()
                },
            ),
            (
                "ubatch_size",
                BackendConfig {
                    ubatch_size: Some(0),
                    ..BackendConfig::default()
                },
            ),
        ] {
            let error = LlamaCppRuntimeSettings::try_from_backend_config(&config)
                .expect_err("zero setting should fail closed");
            assert!(
                error.to_string().contains(field_name),
                "expected {field_name} in {error}"
            );
        }
    }

    #[test]
    fn llamacpp_runtime_settings_reject_invalid_device_selectors() {
        for (device, expected) in [
            (None, "device setting is required"),
            (
                Some(
                    BackendStartupDeviceIntent::canonical_device_id("cuda:0")
                        .expect("valid canonical device id"),
                ),
                "does not accept canonical device id",
            ),
            (
                Some(BackendStartupDeviceIntent::scheduler_policy(
                    InferenceDevicePolicy::Explicit {
                        device_class: InferenceDeviceClass::Cuda,
                        device_id: None,
                    },
                )),
                "unresolved explicit scheduler policy",
            ),
        ] {
            let error = LlamaCppRuntimeSettings::try_from_backend_config(&BackendConfig {
                device,
                ..BackendConfig::default()
            })
            .expect_err("invalid device selector should fail closed");
            assert!(
                error.to_string().contains(expected),
                "expected {expected:?} in {error}"
            );
        }
    }

    #[test]
    fn chat_chunk_serde_omits_absent_usage() {
        let chunk = ChatChunk {
            content: Some("hello".to_string()),
            done: false,
            usage: None,
            cache_handle_id: None,
        };

        let encoded = serde_json::to_value(&chunk).expect("chat chunk serializes");

        assert_eq!(encoded["content"], serde_json::json!("hello"));
        assert_eq!(encoded["done"], serde_json::json!(false));
        assert!(
            encoded.get("usage").is_none(),
            "absent usage should stay omitted for append-only compatibility"
        );
        assert!(
            encoded.get("cache_handle_id").is_none(),
            "absent cache handle should stay omitted for append-only compatibility"
        );
    }

    #[test]
    fn chat_chunk_serde_keeps_bounded_usage_counts() {
        let chunk = ChatChunk {
            content: None,
            done: true,
            usage: Some(InferenceUsage {
                prompt_tokens: Some(8),
                completion_tokens: Some(5),
                total_tokens: Some(13),
            }),
            cache_handle_id: Some("kv-checkpoint-1".to_string()),
        };

        let encoded = serde_json::to_value(&chunk).expect("chat chunk serializes");

        assert_eq!(encoded["content"], serde_json::Value::Null);
        assert_eq!(encoded["done"], serde_json::json!(true));
        assert_eq!(encoded["usage"]["prompt_tokens"], serde_json::json!(8));
        assert_eq!(encoded["usage"]["completion_tokens"], serde_json::json!(5));
        assert_eq!(encoded["usage"]["total_tokens"], serde_json::json!(13));
        assert_eq!(
            encoded["cache_handle_id"],
            serde_json::json!("kv-checkpoint-1")
        );
    }

    #[test]
    fn openai_embedding_token_count_uses_single_item_prompt_usage() {
        let response = serde_json::json!({
            "usage": {
                "prompt_tokens": 9,
                "total_tokens": 9
            }
        });

        assert_eq!(
            openai_embedding_token_count_for_single_result(&response, 1),
            9
        );
    }

    #[test]
    fn openai_embedding_token_count_uses_total_tokens_fallback() {
        let response = serde_json::json!({
            "usage": {
                "total_tokens": 7
            }
        });

        assert_eq!(
            openai_embedding_token_count_for_single_result(&response, 1),
            7
        );
    }

    #[test]
    fn openai_embedding_token_count_avoids_multi_item_attribution() {
        let response = serde_json::json!({
            "usage": {
                "prompt_tokens": 9,
                "total_tokens": 9
            }
        });

        assert_eq!(
            openai_embedding_token_count_for_single_result(&response, 2),
            0
        );
    }

    #[test]
    fn openai_embedding_token_count_drops_oversized_counts() {
        let response = serde_json::json!({
            "usage": {
                "prompt_tokens": u64::MAX
            }
        });

        assert_eq!(
            openai_embedding_token_count_for_single_result(&response, 1),
            0
        );
    }
}
