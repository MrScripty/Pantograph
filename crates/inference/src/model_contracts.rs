//! Transformers-aligned model and task contracts consumed by inference.
//!
//! These contracts describe model-package facts and request semantics without
//! selecting a live runtime. Pumas or fixture producers can populate them, while
//! runtime registry and scheduler layers remain responsible for final placement,
//! admission, and policy decisions.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current inference package-facts contract version.
pub const MODEL_PACKAGE_FACTS_CONTRACT_VERSION: u32 = 1;

/// Compact execution descriptor mirrored from Pumas API output.
///
/// This is intentionally smaller than `ResolvedModelPackageFacts`: it is enough
/// to identify the executable entry point and broad backend hints, but it does
/// not carry tokenizer, processor, generation, provenance, or compatibility
/// detail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ModelExecutionDescriptor {
    pub execution_contract_version: u32,
    pub model_id: String,
    pub entry_path: String,
    pub model_type: String,
    pub task_type_primary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_engine_hints: Vec<String>,
    pub storage_kind: ModelExecutionStorageKind,
    pub validation_state: ModelExecutionValidationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_resolution: Option<Value>,
}

/// Storage kind labels used by the compact Pumas execution descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelExecutionStorageKind {
    LibraryOwned,
    ExternalReference,
    Unknown,
}

/// Validation labels used by the compact Pumas execution descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelExecutionValidationState {
    Valid,
    Degraded,
    Invalid,
    Unknown,
}

/// Stable model reference resolved from the model library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PumasModelRef {
    /// Canonical model id assigned by Pumas or an equivalent model library.
    pub model_id: String,
    /// Optional artifact id when a model exposes multiple executable artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    /// Optional source revision or immutable package revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

/// Model artifact kind exposed by the model library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelArtifactKind {
    Gguf,
    HfCompatibleDirectory,
    Safetensors,
    DiffusersBundle,
    Onnx,
    Adapter,
    Unknown,
}

/// Durable storage location class for a resolved model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStorageKind {
    LocalPath,
    ManagedLibrary,
    RemoteReference,
    Unknown,
}

/// Validation state for the selected model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelValidationState {
    Valid,
    Warning,
    Invalid,
    Stale,
    Unknown,
}

/// Input or output modality used by task signatures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InferenceModality {
    Text,
    Image,
    Audio,
    Video,
    Embedding,
    Tokens,
    Json,
    PointCloud,
    Mesh,
    Other,
}

/// Canonical inference task id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InferenceTaskId {
    TextGeneration,
    ChatCompletion,
    Embedding,
    Rerank,
    ImageGeneration,
    ImageUnderstanding,
    AudioTranscription,
    VideoUnderstanding,
    MultimodalGeneration,
    Unknown,
}

/// Task input/output shape independent of a concrete backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TaskModalitySignature {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<InferenceModality>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<InferenceModality>,
}

impl TaskModalitySignature {
    /// Build a modality signature from explicit input and output modality lists.
    #[must_use]
    pub fn new(inputs: Vec<InferenceModality>, outputs: Vec<InferenceModality>) -> Self {
        Self { inputs, outputs }
    }
}

/// Support tier for a task or backend mapping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SupportTier {
    Stable,
    Experimental,
    Roadmap,
    Unsupported,
    Unknown,
}

/// Registry entry for a canonical inference task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TaskRegistryEntry {
    pub task_id: InferenceTaskId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    pub modality_signature: TaskModalitySignature,
    pub result_family: String,
    pub support_tier: SupportTier,
}

/// Upstream task evidence discovered from a model package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TaskEvidence {
    pub task_id: InferenceTaskId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream_labels: Vec<String>,
    pub modality_signature: TaskModalitySignature,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Backend-family hint discovered from package facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendHintFact {
    pub backend_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_family: Option<String>,
    #[serde(default)]
    pub source: BackendHintSource,
    #[serde(default)]
    pub executable_guarantee: bool,
}

/// Source class for a backend hint.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendHintSource {
    PackageMetadata,
    FilePattern,
    RemoteSearchTag,
    UserMetadata,
    #[default]
    Unknown,
}

/// Parsed component presence for a model package.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ModelComponentFacts {
    #[serde(default)]
    pub tokenizer: ComponentState,
    #[serde(default)]
    pub processor: ComponentState,
    #[serde(default)]
    pub image_processor: ComponentState,
    #[serde(default)]
    pub audio_processor: ComponentState,
    #[serde(default)]
    pub video_processor: ComponentState,
    #[serde(default)]
    pub chat_template: ComponentState,
    #[serde(default)]
    pub generation_config: ComponentState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

/// Presence/parse state for a model package component.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentState {
    Present,
    Missing,
    Invalid,
    NotRequired,
    #[default]
    Unknown,
}

/// Model-provided generation defaults, separate from user request overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GenerationDefaultFacts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<LengthGenerationOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingGenerationOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search: Option<SearchGenerationOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopping: Option<StoppingGenerationOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheGenerationOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputGenerationOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub special_tokens: Option<SpecialTokenGenerationOptions>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub backend_extensions: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

/// Length-related generation options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct LengthGenerationOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_new_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_new_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

/// Sampling-related generation options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct SamplingGenerationOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f32>,
}

/// Search/beam-related generation options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SearchGenerationOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_beams: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_return_sequences: Option<u32>,
}

/// Stopping-related generation options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct StoppingGenerationOptions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_strings: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub eos_token_ids: Vec<u32>,
}

/// Cache-related generation options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CacheGenerationOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cache: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kv_cache_checkpoint_requested: Option<bool>,
}

/// Output-detail generation options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct OutputGenerationOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_logprobs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_token_ids: Option<bool>,
}

/// Special-token generation behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SpecialTokenGenerationOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bos_token_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eos_token_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pad_token_id: Option<u32>,
}

/// Backend support result for a requested generation option.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct OptionCompatibilityDiagnostic {
    pub option_path: String,
    pub state: OptionSupportState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Support state for a generation option at a backend boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptionSupportState {
    Honored,
    Mapped,
    Defaulted,
    Ignored,
    Unsupported,
    Rejected,
    ModelUnavailable,
    BackendUnavailable,
}

/// Security and trust facts discovered from a package.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CustomCodeFacts {
    #[serde(default)]
    pub requires_trust: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auto_map_entries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_manifests: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

/// Resource estimate attached to an advisory feasible execution candidate.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ResourceEstimate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disk_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ram_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vram_bytes: Option<u64>,
}

/// Advisory model-library feasibility fact for a backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct FeasibleExecutionCandidate {
    pub backend_key: String,
    pub task_id: InferenceTaskId,
    #[serde(default)]
    pub feasible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default)]
    pub resource_estimate: ResourceEstimate,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclusion_reasons: Vec<String>,
}

/// Resolved package facts consumed by inference and higher-level preflight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedModelPackageFacts {
    pub contract_version: u32,
    pub model_ref: PumasModelRef,
    pub artifact_kind: ModelArtifactKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_path: Option<String>,
    pub storage_kind: ModelStorageKind,
    pub validation_state: ModelValidationState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_evidence: Vec<TaskEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backend_hints: Vec<BackendHintFact>,
    #[serde(default)]
    pub components: ModelComponentFacts,
    #[serde(default)]
    pub generation_defaults: GenerationDefaultFacts,
    #[serde(default)]
    pub custom_code: CustomCodeFacts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feasible_execution_candidates: Vec<FeasibleExecutionCandidate>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub provenance: Value,
}

impl ResolvedModelPackageFacts {
    /// Whether this fact payload matches the crate's current contract version.
    #[must_use]
    pub fn uses_current_contract(&self) -> bool {
        self.contract_version == MODEL_PACKAGE_FACTS_CONTRACT_VERSION
    }
}

/// Lifecycle phase reported by inference without exposing backend internals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InferenceLifecyclePhase {
    ModelPackageResolution,
    TaskValidation,
    Preprocessing,
    BackendExecution,
    Postprocessing,
    ResultProjection,
}

/// Host-agnostic Pumas model-library change event consumed by caches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PumasModelLibraryChangeEvent {
    pub update_cursor: String,
    pub kind: PumasModelLibraryChangeKind,
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact_family: Option<String>,
    #[serde(default)]
    pub refresh_summary: bool,
    #[serde(default)]
    pub refresh_details: bool,
}

/// Kind of model-library change that invalidates consumer cache projections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PumasModelLibraryChangeKind {
    ModelAdded,
    ModelRemoved,
    MetadataModified,
    PackageFactsModified,
    StaleFactsInvalidated,
    DependencyBindingModified,
}
