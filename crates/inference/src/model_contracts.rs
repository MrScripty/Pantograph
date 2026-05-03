//! Transformers-aligned model and task contracts consumed by inference.
//!
//! These contracts describe model-package facts and request semantics without
//! selecting a live runtime. Pumas or fixture producers can populate them, while
//! runtime registry and scheduler layers remain responsible for final placement,
//! admission, and policy decisions.

use std::collections::BTreeMap;

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
    /// Optional source revision or immutable package revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Optional selected artifact id when a model exposes multiple artifacts.
    #[serde(
        default,
        alias = "artifact_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub selected_artifact_id: Option<String>,
    /// Optional selected artifact path returned during legacy-reference migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_artifact_path: Option<String>,
    /// Bounded diagnostics emitted while migrating legacy references to Pumas refs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migration_diagnostics: Vec<ModelRefMigrationDiagnostic>,
}

/// Diagnostic produced while converting a legacy model reference to a Pumas ref.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ModelRefMigrationDiagnostic {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
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
    Shard,
    Unknown,
}

/// Durable storage location class for a resolved model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStorageKind {
    #[serde(alias = "managed_library")]
    LibraryOwned,
    #[serde(alias = "local_path", alias = "remote_reference")]
    ExternalReference,
    Unknown,
}

/// Validation state for the selected model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelValidationState {
    Valid,
    #[serde(alias = "warning", alias = "stale")]
    Degraded,
    Invalid,
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

impl Default for SupportTier {
    fn default() -> Self {
        Self::Unknown
    }
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TaskEvidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type_primary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_modalities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_modalities: Vec<String>,
}

/// Stable backend hint labels Pumas may expose as advisory package facts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BackendHintLabel {
    Transformers,
    #[serde(rename = "llama.cpp")]
    LlamaCpp,
    Vllm,
    Mlx,
    Candle,
    Diffusers,
    OnnxRuntime,
}

/// Backend hints as advisory package facts, not executable runtime decisions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendHintFacts {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted: Vec<BackendHintLabel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unsupported: Vec<String>,
}

/// Normalized backend-family hint derived by Pantograph from package facts.
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

/// Package component kind with stable labels for consumer diagnostics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessorComponentKind {
    Config,
    Tokenizer,
    TokenizerConfig,
    SpecialTokensMap,
    Processor,
    Preprocessor,
    ImageProcessor,
    VideoProcessor,
    AudioFeatureExtractor,
    FeatureExtractor,
    ChatTemplate,
    GenerationConfig,
    ModelIndex,
    WeightIndex,
    Shard,
    Weights,
    Adapter,
    Quantization,
    Other,
}

/// Component-level package-file evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ProcessorComponentFacts {
    pub kind: ProcessorComponentKind,
    pub status: PackageFactStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Transformers/Hugging Face package layout evidence.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TransformersPackageEvidence {
    pub config_status: PackageFactStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_model_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub architectures: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dtype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub torch_dtype: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auto_map: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor_class: Option<String>,
    pub generation_config_status: PackageFactStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_repo_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_files: Vec<String>,
}

/// Parsed component presence derived by Pantograph from package facts.
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
    Unsupported,
    Uninspected,
    NotRequired,
    #[default]
    Unknown,
}

/// Pumas package-fact status for summary rows and compact component evidence.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageFactStatus {
    Present,
    Missing,
    Invalid,
    Unsupported,
    #[default]
    Uninspected,
}

/// Model-provided generation defaults, separate from user request overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GenerationDefaultFacts {
    pub status: PackageFactStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ModelPackageDiagnostic>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
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

/// Complete generation option groups for a canonical inference request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GenerationOptions {
    #[serde(default)]
    pub length: LengthGenerationOptions,
    #[serde(default)]
    pub sampling: SamplingGenerationOptions,
    #[serde(default)]
    pub search: SearchGenerationOptions,
    #[serde(default)]
    pub stopping: StoppingGenerationOptions,
    #[serde(default)]
    pub cache: CacheGenerationOptions,
    #[serde(default)]
    pub output: OutputGenerationOptions,
    #[serde(default)]
    pub special_tokens: SpecialTokenGenerationOptions,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub backend_extensions: BTreeMap<String, Value>,
}

impl GenerationOptions {
    /// Return canonical option paths for every explicitly requested option.
    ///
    /// Empty vectors and absent optional values are not requested. Backend
    /// mappers can use this list to prove they emitted an option compatibility
    /// diagnostic for every requested option they inspected.
    #[must_use]
    pub fn requested_option_paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        push_requested_option(
            &mut paths,
            "length.max_new_tokens",
            self.length.max_new_tokens,
        );
        push_requested_option(
            &mut paths,
            "length.min_new_tokens",
            self.length.min_new_tokens,
        );
        push_requested_option(&mut paths, "length.max_length", self.length.max_length);
        push_requested_option(
            &mut paths,
            "sampling.temperature",
            self.sampling.temperature,
        );
        push_requested_option(&mut paths, "sampling.top_p", self.sampling.top_p);
        push_requested_option(&mut paths, "sampling.top_k", self.sampling.top_k);
        push_requested_option(
            &mut paths,
            "sampling.repetition_penalty",
            self.sampling.repetition_penalty,
        );
        push_requested_option(&mut paths, "sampling.seed", self.sampling.seed);
        push_requested_option(&mut paths, "search.num_beams", self.search.num_beams);
        push_requested_option(
            &mut paths,
            "search.num_return_sequences",
            self.search.num_return_sequences,
        );
        push_requested_vec_option(
            &mut paths,
            "stopping.stop_strings",
            &self.stopping.stop_strings,
        );
        push_requested_vec_option(
            &mut paths,
            "stopping.eos_token_ids",
            &self.stopping.eos_token_ids,
        );
        push_requested_option(&mut paths, "cache.use_cache", self.cache.use_cache);
        push_requested_option(
            &mut paths,
            "cache.kv_cache_checkpoint_requested",
            self.cache.kv_cache_checkpoint_requested,
        );
        push_requested_option(
            &mut paths,
            "output.return_logprobs",
            self.output.return_logprobs,
        );
        push_requested_option(
            &mut paths,
            "output.return_token_ids",
            self.output.return_token_ids,
        );
        push_requested_option(
            &mut paths,
            "special_tokens.bos_token_id",
            self.special_tokens.bos_token_id,
        );
        push_requested_option(
            &mut paths,
            "special_tokens.eos_token_id",
            self.special_tokens.eos_token_id,
        );
        push_requested_option(
            &mut paths,
            "special_tokens.pad_token_id",
            self.special_tokens.pad_token_id,
        );
        paths.extend(
            self.backend_extensions
                .keys()
                .map(|key| format!("backend_extensions.{key}")),
        );
        paths
    }

    /// Resolve generation options from layered defaults and request overrides.
    ///
    /// Precedence is model defaults, then workflow/node defaults, then runtime
    /// preset, then request overrides. Missing fields do not override earlier
    /// layers. The returned diagnostics identify the layer that supplied each
    /// resolved option value; they do not make backend support decisions.
    #[must_use]
    pub fn resolve_precedence(
        model_defaults: Option<&Value>,
        workflow_defaults: Option<&Self>,
        runtime_preset: Option<&Self>,
        request_overrides: Option<&Self>,
    ) -> GenerationOptionResolutionReport {
        let mut options = GenerationOptions::default();
        let mut diagnostics = Vec::new();

        if let Some(defaults) = model_defaults {
            let parsed = Self::from_generation_defaults_value(defaults);
            options.apply_layer(
                &parsed,
                GenerationOptionSource::ModelDefaults,
                OptionSupportState::Defaulted,
                &mut diagnostics,
            );
        }
        if let Some(defaults) = workflow_defaults {
            options.apply_layer(
                defaults,
                GenerationOptionSource::WorkflowDefaults,
                OptionSupportState::Defaulted,
                &mut diagnostics,
            );
        }
        if let Some(preset) = runtime_preset {
            options.apply_layer(
                preset,
                GenerationOptionSource::RuntimePreset,
                OptionSupportState::Defaulted,
                &mut diagnostics,
            );
        }
        if let Some(overrides) = request_overrides {
            options.apply_layer(
                overrides,
                GenerationOptionSource::RequestOverride,
                OptionSupportState::Honored,
                &mut diagnostics,
            );
        }

        GenerationOptionResolutionReport {
            options,
            diagnostics,
        }
    }

    fn apply_layer(
        &mut self,
        layer: &Self,
        source: GenerationOptionSource,
        state: OptionSupportState,
        diagnostics: &mut Vec<GenerationOptionResolutionDiagnostic>,
    ) {
        apply_optional_option(
            &mut self.length.max_new_tokens,
            layer.length.max_new_tokens,
            "length.max_new_tokens",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.length.min_new_tokens,
            layer.length.min_new_tokens,
            "length.min_new_tokens",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.length.max_length,
            layer.length.max_length,
            "length.max_length",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.sampling.temperature,
            layer.sampling.temperature,
            "sampling.temperature",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.sampling.top_p,
            layer.sampling.top_p,
            "sampling.top_p",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.sampling.top_k,
            layer.sampling.top_k,
            "sampling.top_k",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.sampling.repetition_penalty,
            layer.sampling.repetition_penalty,
            "sampling.repetition_penalty",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.sampling.seed,
            layer.sampling.seed,
            "sampling.seed",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.search.num_beams,
            layer.search.num_beams,
            "search.num_beams",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.search.num_return_sequences,
            layer.search.num_return_sequences,
            "search.num_return_sequences",
            source,
            state,
            diagnostics,
        );
        apply_vec_option(
            &mut self.stopping.stop_strings,
            &layer.stopping.stop_strings,
            "stopping.stop_strings",
            source,
            state,
            diagnostics,
        );
        apply_vec_option(
            &mut self.stopping.eos_token_ids,
            &layer.stopping.eos_token_ids,
            "stopping.eos_token_ids",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.cache.use_cache,
            layer.cache.use_cache,
            "cache.use_cache",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.cache.kv_cache_checkpoint_requested,
            layer.cache.kv_cache_checkpoint_requested,
            "cache.kv_cache_checkpoint_requested",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.output.return_logprobs,
            layer.output.return_logprobs,
            "output.return_logprobs",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.output.return_token_ids,
            layer.output.return_token_ids,
            "output.return_token_ids",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.special_tokens.bos_token_id,
            layer.special_tokens.bos_token_id,
            "special_tokens.bos_token_id",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.special_tokens.eos_token_id,
            layer.special_tokens.eos_token_id,
            "special_tokens.eos_token_id",
            source,
            state,
            diagnostics,
        );
        apply_optional_option(
            &mut self.special_tokens.pad_token_id,
            layer.special_tokens.pad_token_id,
            "special_tokens.pad_token_id",
            source,
            state,
            diagnostics,
        );
        for (key, value) in &layer.backend_extensions {
            self.backend_extensions.insert(key.clone(), value.clone());
            diagnostics.push(GenerationOptionResolutionDiagnostic {
                option_path: format!("backend_extensions.{key}"),
                source,
                state,
                message: Some(format!("generation option resolved from {source:?}")),
            });
        }
    }

    fn from_generation_defaults_value(defaults: &Value) -> Self {
        let mut options = Self::default();
        options.length.max_new_tokens = read_u32(defaults, "max_new_tokens");
        options.length.min_new_tokens = read_u32(defaults, "min_new_tokens");
        options.length.max_length = read_u32(defaults, "max_length");
        options.sampling.temperature = read_f32(defaults, "temperature");
        options.sampling.top_p = read_f32(defaults, "top_p");
        options.sampling.top_k = read_u32(defaults, "top_k");
        options.sampling.repetition_penalty = read_f32(defaults, "repetition_penalty");
        options.sampling.seed = read_u64(defaults, "seed");
        options.search.num_beams = read_u32(defaults, "num_beams");
        options.search.num_return_sequences = read_u32(defaults, "num_return_sequences");
        options.stopping.stop_strings = read_string_array(defaults, "stop_strings");
        options.stopping.eos_token_ids = read_u32_array(defaults, "eos_token_ids");
        options.cache.use_cache = read_bool(defaults, "use_cache");
        options.special_tokens.bos_token_id = read_u32(defaults, "bos_token_id");
        options.special_tokens.eos_token_id = read_u32(defaults, "eos_token_id");
        options.special_tokens.pad_token_id = read_u32(defaults, "pad_token_id");
        options
    }
}

/// Source layer that supplied a resolved generation option value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationOptionSource {
    ModelDefaults,
    WorkflowDefaults,
    RuntimePreset,
    RequestOverride,
}

/// One resolved generation option source decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct GenerationOptionResolutionDiagnostic {
    pub option_path: String,
    pub source: GenerationOptionSource,
    pub state: OptionSupportState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Resolved generation options plus bounded source diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct GenerationOptionResolutionReport {
    pub options: GenerationOptions,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<GenerationOptionResolutionDiagnostic>,
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptionSupportState {
    Honored,
    Mapped,
    Defaulted,
    Ignored,
    Unsupported,
    Rejected,
    Conflict,
    ModelUnavailable,
    BackendUnavailable,
    RequiresModelSupport,
    RequiresBackendSupport,
}

fn apply_optional_option<T: Copy>(
    target: &mut Option<T>,
    value: Option<T>,
    option_path: &'static str,
    source: GenerationOptionSource,
    state: OptionSupportState,
    diagnostics: &mut Vec<GenerationOptionResolutionDiagnostic>,
) {
    if let Some(value) = value {
        *target = Some(value);
        diagnostics.push(GenerationOptionResolutionDiagnostic {
            option_path: option_path.to_string(),
            source,
            state,
            message: Some(format!("generation option resolved from {source:?}")),
        });
    }
}

fn push_requested_option<T>(paths: &mut Vec<String>, option_path: &'static str, value: Option<T>) {
    if value.is_some() {
        paths.push(option_path.to_string());
    }
}

fn push_requested_vec_option<T>(paths: &mut Vec<String>, option_path: &'static str, value: &[T]) {
    if !value.is_empty() {
        paths.push(option_path.to_string());
    }
}

fn apply_vec_option<T: Clone>(
    target: &mut Vec<T>,
    value: &[T],
    option_path: &'static str,
    source: GenerationOptionSource,
    state: OptionSupportState,
    diagnostics: &mut Vec<GenerationOptionResolutionDiagnostic>,
) {
    if !value.is_empty() {
        *target = value.to_vec();
        diagnostics.push(GenerationOptionResolutionDiagnostic {
            option_path: option_path.to_string(),
            source,
            state,
            message: Some(format!("generation option resolved from {source:?}")),
        });
    }
}

fn read_u32(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn read_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn read_f32(value: &Value, key: &str) -> Option<f32> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
}

fn read_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn read_string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn read_u32_array(value: &Value, key: &str) -> Vec<u32> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .filter_map(|value| u32::try_from(value).ok())
        .collect()
}

/// Security and trust facts discovered from a package.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CustomCodeFacts {
    pub requires_custom_code: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_code_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auto_map_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub class_references: Vec<PackageClassReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_manifests: Vec<String>,
}

/// Class reference discovered from package metadata without importing code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct PackageClassReference {
    pub kind: ProcessorComponentKind,
    pub class_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

/// Current-state validation finding for a resolved artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AssetValidationError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Artifact-specific package evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedArtifactFacts {
    pub artifact_kind: ModelArtifactKind,
    pub entry_path: String,
    pub storage_kind: ModelStorageKind,
    pub validation_state: ModelValidationState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_errors: Vec<AssetValidationError>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub companion_artifacts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sibling_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_files: Vec<String>,
}

/// Generic package-fact diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ModelPackageDiagnostic {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Resolved package facts consumed by inference and higher-level preflight.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedModelPackageFacts {
    pub package_facts_contract_version: u32,
    pub model_ref: PumasModelRef,
    pub artifact: ResolvedArtifactFacts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ProcessorComponentFacts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transformers: Option<TransformersPackageEvidence>,
    pub task: TaskEvidence,
    pub generation_defaults: GenerationDefaultFacts,
    pub custom_code: CustomCodeFacts,
    pub backend_hints: BackendHintFacts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ModelPackageDiagnostic>,
}

impl ResolvedModelPackageFacts {
    /// Whether this fact payload matches the crate's current contract version.
    #[must_use]
    pub fn uses_current_contract(&self) -> bool {
        self.package_facts_contract_version == MODEL_PACKAGE_FACTS_CONTRACT_VERSION
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

/// Fact family that changed in the Pumas model library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelFactFamily {
    ModelRecord,
    Metadata,
    PackageFacts,
    DependencyBindings,
    Validation,
    SearchIndex,
}

/// Kind of Pumas model-library change that invalidates consumer projections.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelLibraryChangeKind {
    ModelAdded,
    ModelRemoved,
    MetadataModified,
    PackageFactsModified,
    StaleFactsInvalidated,
    DependencyBindingModified,
}

/// Consumer refresh scope implied by a Pumas model-library update event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelLibraryRefreshScope {
    Summary,
    Detail,
    SummaryAndDetail,
}

/// Host-agnostic Pumas model-library update event consumed by caches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ModelLibraryUpdateEvent {
    pub cursor: String,
    pub model_id: String,
    pub change_kind: ModelLibraryChangeKind,
    pub fact_family: ModelFactFamily,
    pub refresh_scope: ModelLibraryRefreshScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_revision: Option<String>,
}

impl ModelLibraryUpdateEvent {
    #[must_use]
    pub fn refreshes_summary(&self) -> bool {
        matches!(
            self.refresh_scope,
            ModelLibraryRefreshScope::Summary | ModelLibraryRefreshScope::SummaryAndDetail
        )
    }

    #[must_use]
    pub fn refreshes_details(&self) -> bool {
        matches!(
            self.refresh_scope,
            ModelLibraryRefreshScope::Detail | ModelLibraryRefreshScope::SummaryAndDetail
        )
    }
}

/// Ordered page of Pumas model-library updates after a consumer cursor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ModelLibraryUpdateFeed {
    pub cursor: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<ModelLibraryUpdateEvent>,
    pub stale_cursor: bool,
    pub snapshot_required: bool,
}

/// Consumer-visible freshness/source state for a package-facts summary row.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelPackageFactsSummaryStatus {
    Cached,
    Missing,
    Invalid,
    Fresh,
    DetailDerived,
    Regenerated,
}

/// Compact package-fact summary intended for indexing, list views, and stale checks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedModelPackageFactsSummary {
    pub package_facts_contract_version: u32,
    pub model_ref: PumasModelRef,
    pub artifact_kind: ModelArtifactKind,
    pub entry_path: String,
    pub storage_kind: ModelStorageKind,
    pub validation_state: ModelValidationState,
    pub task: TaskEvidence,
    pub backend_hints: BackendHintFacts,
    pub requires_custom_code: bool,
    pub config_status: PackageFactStatus,
    pub tokenizer_status: PackageFactStatus,
    pub processor_status: PackageFactStatus,
    pub generation_config_status: PackageFactStatus,
    pub generation_defaults_status: PackageFactStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostic_codes: Vec<String>,
}

/// Single model package-facts summary lookup result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ModelPackageFactsSummaryResult {
    pub model_id: String,
    pub status: ModelPackageFactsSummaryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<ResolvedModelPackageFactsSummary>,
}

/// Startup/list snapshot item for host cache population.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ModelPackageFactsSummarySnapshotItem {
    pub model_id: String,
    pub status: ModelPackageFactsSummaryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<ResolvedModelPackageFactsSummary>,
}

/// Bounded startup snapshot of cached package-facts summaries plus update cursor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ModelPackageFactsSummarySnapshot {
    pub cursor: String,
    pub items: Vec<ModelPackageFactsSummarySnapshotItem>,
}
