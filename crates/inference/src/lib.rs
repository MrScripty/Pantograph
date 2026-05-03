//! Multi-backend AI inference library
//!
//! This library provides a unified interface for different AI inference backends:
//! - **llama.cpp**: Local inference via GGUF models (default)
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
pub mod managed_binaries;
pub mod managed_media_dependencies;
pub mod managed_redistributables;
pub mod managed_runtime;
pub mod model_contracts;
pub mod process;
pub mod server;
pub mod types;

// Re-exports for convenience
pub use backend::{
    BackendCapabilities, BackendCapabilityFacts, BackendCompatibilityIssue,
    BackendCompatibilityIssueKind, BackendCompatibilityOptions, BackendCompatibilityReport,
    BackendCompatibilityRequest, BackendCompatibilityStatus, BackendComponentCapability,
    BackendConfig, BackendError, BackendFactory, BackendFeatureCapabilityFacts,
    BackendFeatureSupport, BackendInfo, BackendModelSourceCapabilityFacts, BackendRegistry,
    BackendRequestCancellationSemantics, BackendRequestCleanupSemantics,
    BackendRequestLifecycleFacts, BackendRequestLifecyclePhaseFacts, BackendTaskCapability,
    ChatChunk, EmbeddingResult, InferenceBackend,
};

#[cfg(feature = "backend-llamacpp")]
pub use backend::LlamaCppBackend;

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
pub use managed_binaries::{
    list_managed_binary_statuses, resolve_managed_binary_command, ManagedBinaryActionSupport,
    ManagedBinaryCategory, ManagedBinaryFacadeError, ManagedBinaryKey, ManagedBinarySource,
    ManagedBinaryStatus, ManagedBinaryVersionStatus,
};
pub use managed_media_dependencies::{
    acquire_media_conversion_dependency_plan, format_media_conversion_dependency_lease_holder,
    media_conversion_dependency_lease_holder_convention, open_color_io_activation_validation_state,
    release_media_conversion_dependency_plan, resolve_media_conversion_dependency_executable_path,
    validate_media_conversion_dependency_lease_holder, validate_open_color_io_activation,
    MediaConversionDependency, MediaConversionDependencyId, MediaConversionDependencyLease,
    MediaConversionDependencyLeaseToken, MediaConversionDependencyPlan,
    MediaConversionDependencyPlanRequest, MediaConversionJobKind, OpenColorIoActivation,
    OpenColorIoActivationValidation, OpenColorIoActivationValidationState,
};
pub use managed_redistributables::{
    acquire_managed_redistributable_lease, activate_managed_redistributable_version,
    install_managed_redistributable_from_staging, list_managed_redistributable_statuses,
    load_managed_redistributable_state, managed_redistributable_catalog,
    managed_redistributable_catalog_entry, managed_redistributable_status,
    managed_redistributables_dir, release_managed_redistributable_lease,
    remove_managed_redistributable_version, save_managed_redistributable_state,
    select_managed_redistributable_version, set_default_managed_redistributable_version,
    ManagedRedistributableArchiveKind, ManagedRedistributableCatalogEntry,
    ManagedRedistributableCategory, ManagedRedistributableId, ManagedRedistributableInstallState,
    ManagedRedistributableLease, ManagedRedistributableLeaseToken,
    ManagedRedistributablePackageKind, ManagedRedistributablePersistedDependency,
    ManagedRedistributablePersistedState, ManagedRedistributableReadiness,
    ManagedRedistributableSelection, ManagedRedistributableSource, ManagedRedistributableStatus,
    ManagedRedistributableVersionStatus,
};
pub use managed_runtime::{
    binary_capability, cancel_binary_download, check_binary_status, download_binary,
    list_binary_capabilities, list_managed_runtime_snapshots, load_managed_runtime_state,
    managed_runtime_dir, managed_runtime_snapshot, pause_binary_download,
    reconcile_interrupted_managed_runtime_jobs, refresh_managed_runtime_catalog,
    refresh_managed_runtime_catalogs, remove_binary, remove_binary_version, resolve_binary_command,
    save_managed_runtime_state, select_managed_runtime_version,
    set_default_managed_runtime_version, BinaryStatus, DownloadProgress, ManagedBinaryCapability,
    ManagedBinaryId, ManagedBinaryInstallState, ManagedRuntimeCatalogVersion,
    ManagedRuntimeHistoryEventKind, ManagedRuntimeInstallHistoryEntry,
    ManagedRuntimeJobArtifactStatus, ManagedRuntimeJobState, ManagedRuntimeJobStatus,
    ManagedRuntimePersistedJobArtifact, ManagedRuntimePersistedRuntime,
    ManagedRuntimePersistedState, ManagedRuntimePersistedVersion, ManagedRuntimeReadinessState,
    ManagedRuntimeSelectionState, ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus,
    ResolvedCommand,
};
pub use model_contracts::{
    AssetValidationError, BackendHintFact, BackendHintFacts, BackendHintLabel, BackendHintSource,
    CacheGenerationOptions, ComponentState, CustomCodeFacts, GenerationDefaultFacts,
    GenerationOptionResolutionDiagnostic, GenerationOptionResolutionReport, GenerationOptionSource,
    GenerationOptions, InferenceLifecyclePhase, InferenceModality, InferenceTaskId,
    LengthGenerationOptions, ModelArtifactKind, ModelComponentFacts, ModelExecutionDescriptor,
    ModelExecutionStorageKind, ModelExecutionValidationState, ModelFactFamily,
    ModelLibraryChangeKind, ModelLibraryRefreshScope, ModelLibraryUpdateEvent,
    ModelLibraryUpdateFeed, ModelPackageDiagnostic, ModelPackageFactsSummaryResult,
    ModelPackageFactsSummarySnapshot, ModelPackageFactsSummarySnapshotItem,
    ModelPackageFactsSummaryStatus, ModelRefMigrationDiagnostic, ModelStorageKind,
    ModelValidationState, OptionCompatibilityDiagnostic, OptionSupportState,
    OutputGenerationOptions, PackageClassReference, PackageFactStatus, ProcessorComponentFacts,
    ProcessorComponentKind, PumasModelRef, ResolvedArtifactFacts, ResolvedModelPackageFacts,
    ResolvedModelPackageFactsSummary, SamplingGenerationOptions, SearchGenerationOptions,
    SpecialTokenGenerationOptions, StoppingGenerationOptions, SupportTier, TaskEvidence,
    TaskModalitySignature, TaskRegistryEntry, TransformersPackageEvidence,
    MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
};
pub use process::{ProcessEvent, ProcessHandle, ProcessSpawner};
pub use server::{LlamaServer, ServerMode, SharedLlamaServer};
pub use types::{
    ChatMessage, ChatRequest, ContentPart, Delta, EncodedImage, ImageGenerationRequest,
    ImageGenerationResult, ImageUrlData, InferenceEmbeddingResult, InferenceExecutionInput,
    InferenceExecutionRequest, InferenceExecutionRequestValidationError, InferenceExecutionResult,
    InferenceRequestLifecycleEvent, InferenceRequestLifecycleEventKind,
    InferenceRequestLifecycleEventSink, InferenceUsage, MaskedPrompt, PromptSegment, RerankRequest,
    RerankResponse, RerankResult, RuntimeLifecycleSnapshot, ServerModeInfo, StreamChoice,
    StreamChunk, StreamEvent,
};

#[cfg(feature = "std-process")]
pub use process::StdProcessSpawner;
