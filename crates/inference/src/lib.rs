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
pub mod managed_dependencies;
pub mod managed_media_dependencies;
mod managed_redistributables;
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
    resolve_managed_binary_command, ManagedBinaryFacadeError, ManagedBinaryKey,
};
pub use managed_dependencies::{
    list_all_managed_dependency_statuses, resolve_managed_dependency_command,
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
pub use managed_redistributables::{list_managed_dependency_statuses, managed_dependency_status};
pub use managed_runtime::{
    binary_capability, cancel_binary_download, check_binary_status, download_binary,
    list_binary_capabilities, list_managed_runtime_dependency_statuses,
    list_managed_runtime_snapshots, load_managed_runtime_state, managed_runtime_dependency_status,
    managed_runtime_dir, managed_runtime_snapshot, pause_binary_download,
    reconcile_interrupted_managed_runtime_jobs, refresh_managed_runtime_catalog,
    refresh_managed_runtime_catalogs, remove_binary, remove_binary_version, resolve_binary_command,
    resolve_runtime_sidecar_dependency_command, save_managed_runtime_state,
    select_managed_runtime_version, set_default_managed_runtime_version, BinaryStatus,
    DownloadProgress, ManagedBinaryCapability, ManagedBinaryId, ManagedBinaryInstallState,
    ManagedRuntimeCatalogVersion, ManagedRuntimeHistoryEventKind,
    ManagedRuntimeInstallHistoryEntry, ManagedRuntimeJobArtifactStatus, ManagedRuntimeJobState,
    ManagedRuntimeJobStatus, ManagedRuntimePersistedJobArtifact, ManagedRuntimePersistedRuntime,
    ManagedRuntimePersistedState, ManagedRuntimePersistedVersion, ManagedRuntimeReadinessState,
    ManagedRuntimeSelectionState, ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus,
    ResolvedCommand,
};
pub use model_contracts::{
    default_task_registry_entries, normalize_modality_label, normalize_task_label,
    resolve_task_registry_entry, resolve_task_registry_entry_from_evidence, AssetValidationError,
    BackendHintFact, BackendHintFacts, BackendHintLabel, BackendHintSource, CacheGenerationOptions,
    ComponentState, CustomCodeFacts, GenerationDefaultFacts, GenerationOptionResolutionDiagnostic,
    GenerationOptionResolutionReport, GenerationOptionSource, GenerationOptions,
    InferenceExecutionInputKind, InferenceExecutionResultKind, InferenceLifecyclePhase,
    InferenceModality, InferenceTaskId, LengthGenerationOptions, ModelArtifactKind,
    ModelAuthTokenSource, ModelComponentFacts, ModelExecutionDescriptor, ModelExecutionStorageKind,
    ModelExecutionValidationState, ModelFactFamily, ModelLibraryChangeKind,
    ModelLibraryRefreshScope, ModelLibraryUpdateEvent, ModelLibraryUpdateFeed,
    ModelLoadCachePolicy, ModelLoadNetworkPolicy, ModelLoadSecurityPolicy, ModelPackageDiagnostic,
    ModelPackageFactsSummaryResult, ModelPackageFactsSummarySnapshot,
    ModelPackageFactsSummarySnapshotItem, ModelPackageFactsSummaryStatus,
    ModelRefMigrationDiagnostic, ModelRemoteCodePolicy, ModelStorageKind, ModelValidationState,
    OptionCompatibilityDiagnostic, OptionSupportState, OutputGenerationOptions,
    PackageClassReference, PackageFactStatus, ProcessorComponentFacts, ProcessorComponentKind,
    PumasModelRef, ResolvedArtifactFacts, ResolvedModelPackageFacts,
    ResolvedModelPackageFactsSummary, ResolvedModelSource, ResolvedModelSourceKind,
    SamplingGenerationOptions, SearchGenerationOptions, SpecialTokenGenerationOptions,
    StoppingGenerationOptions, SupportTier, TaskEvidence, TaskExecutionBehavior, TaskFamily,
    TaskModalitySignature, TaskRegistryEntry, TaskRegistryResolutionDiagnostic,
    TaskRegistryResolutionDiagnosticKind, TaskRequestContract, TaskStreamingSupport,
    TransformersPackageEvidence, MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
};
pub use pantograph_managed_dependencies::{
    ManagedDependencyActivation, ManagedDependencyActivationValidationState,
    ManagedDependencyCategory, ManagedDependencyDescriptor, ManagedDependencyInstallState,
    ManagedDependencyKey, ManagedDependencyOperation, ManagedDependencyOperationScope,
    ManagedDependencyReadinessState, ManagedDependencySelectionState, ManagedDependencySource,
    ManagedDependencyStatus, ManagedDependencyVersionStatus, MediaToolDependencyId,
    NativeArtifactDependencyId, ResolvedManagedDependencyCommand, RuntimeSidecarDependencyId,
};
pub use process::{ProcessEvent, ProcessHandle, ProcessSpawner};
pub use server::{LlamaServer, ServerMode, SharedLlamaServer};
pub use types::{
    bounded_inference_artifact_ref, looks_like_local_artifact_ref, AudioTranscriptionRequest,
    AudioTranscriptionResult, AudioTranscriptionSegment, ChatMessage, ChatRequest, ContentPart,
    Delta, EncodedAudio, EncodedImage, ImageGenerationRequest, ImageGenerationResult, ImageUrlData,
    InferenceCompatibilityIssueSummary, InferenceCompatibilityReportSummary,
    InferenceEmbeddingResult, InferenceExecutionInput, InferenceExecutionRequest,
    InferenceExecutionRequestValidationError, InferenceExecutionResult,
    InferenceRequestLifecycleEvent, InferenceRequestLifecycleEventKind,
    InferenceRequestLifecycleEventSink, InferenceRequestLifecycleEventSinkError, InferenceUsage,
    MaskedPrompt, PromptSegment, RerankRequest, RerankResponse, RerankResult,
    RuntimeLifecycleSnapshot, ServerModeInfo, StreamChoice, StreamChunk, StreamEvent,
};

#[cfg(feature = "std-process")]
pub use process::StdProcessSpawner;
