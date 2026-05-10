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
pub const MODEL_PACKAGE_FACTS_CONTRACT_VERSION: u32 = 2;

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

impl InferenceModality {
    /// Stable snake_case label used for modality evidence matching.
    #[must_use]
    pub fn canonical_label(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Embedding => "embedding",
            Self::Tokens => "tokens",
            Self::Json => "json",
            Self::PointCloud => "point_cloud",
            Self::Mesh => "mesh",
            Self::Other => "other",
        }
    }
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
    DepthEstimation,
    AudioTranscription,
    VideoUnderstanding,
    MultimodalGeneration,
    Unknown,
}

impl InferenceTaskId {
    /// Stable snake_case label used across Rust, Python worker envelopes,
    /// workflow node data, and diagnostics.
    #[must_use]
    pub fn canonical_label(&self) -> &'static str {
        match self {
            Self::TextGeneration => "text_generation",
            Self::ChatCompletion => "chat_completion",
            Self::Embedding => "embedding",
            Self::Rerank => "rerank",
            Self::ImageGeneration => "image_generation",
            Self::ImageUnderstanding => "image_understanding",
            Self::DepthEstimation => "depth_estimation",
            Self::AudioTranscription => "audio_transcription",
            Self::VideoUnderstanding => "video_understanding",
            Self::MultimodalGeneration => "multimodal_generation",
            Self::Unknown => "unknown",
        }
    }
}

/// Canonical typed execution input payload kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InferenceExecutionInputKind {
    TextGeneration,
    Embedding,
    Rerank,
    ImageGeneration,
    ImageUnderstanding,
    DepthEstimation,
    AudioTranscription,
    VideoUnderstanding,
    MultimodalGeneration,
}

impl InferenceExecutionInputKind {
    /// Stable snake_case label used in typed request diagnostics.
    #[must_use]
    pub fn canonical_label(self) -> &'static str {
        match self {
            Self::TextGeneration => "text_generation",
            Self::Embedding => "embedding",
            Self::Rerank => "rerank",
            Self::ImageGeneration => "image_generation",
            Self::ImageUnderstanding => "image_understanding",
            Self::DepthEstimation => "depth_estimation",
            Self::AudioTranscription => "audio_transcription",
            Self::VideoUnderstanding => "video_understanding",
            Self::MultimodalGeneration => "multimodal_generation",
        }
    }
}

/// Canonical typed execution result payload kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InferenceExecutionResultKind {
    TextGeneration,
    Embedding,
    Rerank,
    ImageGeneration,
    ImageUnderstanding,
    DepthEstimation,
    AudioTranscription,
    VideoUnderstanding,
    MultimodalGeneration,
}

impl InferenceExecutionResultKind {
    /// Stable snake_case label used in result contracts and diagnostics.
    #[must_use]
    pub fn canonical_label(self) -> &'static str {
        match self {
            Self::TextGeneration => "text_generation",
            Self::Embedding => "embedding",
            Self::Rerank => "rerank",
            Self::ImageGeneration => "image_generation",
            Self::ImageUnderstanding => "image_understanding",
            Self::DepthEstimation => "depth_estimation",
            Self::AudioTranscription => "audio_transcription",
            Self::VideoUnderstanding => "video_understanding",
            Self::MultimodalGeneration => "multimodal_generation",
        }
    }
}

/// Typed request/result payload contract for a canonical task.
///
/// This is a semantic compatibility contract, not a backend route table.
/// Backends can map the same payload kind to Transformers, llama.cpp, vLLM,
/// MLX, Candle, or another adapter internally.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TaskRequestContract {
    pub task_id: InferenceTaskId,
    pub input_kind: InferenceExecutionInputKind,
    pub result_kind: InferenceExecutionResultKind,
    pub execution_supported: bool,
    pub streaming_support: TaskStreamingSupport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_input_modalities: Vec<InferenceModality>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_modalities: Vec<InferenceModality>,
}

impl TaskRequestContract {
    /// Build the canonical request/result contract for a task id.
    #[must_use]
    pub fn for_task(task_id: &InferenceTaskId) -> Option<Self> {
        let contract = match task_id {
            InferenceTaskId::TextGeneration => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::TextGeneration,
                result_kind: InferenceExecutionResultKind::TextGeneration,
                execution_supported: true,
                streaming_support: TaskStreamingSupport::BackendDependent,
                required_input_modalities: vec![InferenceModality::Text],
                output_modalities: vec![InferenceModality::Text],
            },
            InferenceTaskId::ChatCompletion => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::TextGeneration,
                result_kind: InferenceExecutionResultKind::TextGeneration,
                execution_supported: true,
                streaming_support: TaskStreamingSupport::BackendDependent,
                required_input_modalities: vec![InferenceModality::Text],
                output_modalities: vec![InferenceModality::Text],
            },
            InferenceTaskId::Embedding => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::Embedding,
                result_kind: InferenceExecutionResultKind::Embedding,
                execution_supported: true,
                streaming_support: TaskStreamingSupport::Unsupported,
                required_input_modalities: vec![InferenceModality::Text],
                output_modalities: vec![InferenceModality::Embedding],
            },
            InferenceTaskId::Rerank => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::Rerank,
                result_kind: InferenceExecutionResultKind::Rerank,
                execution_supported: true,
                streaming_support: TaskStreamingSupport::Unsupported,
                required_input_modalities: vec![InferenceModality::Text, InferenceModality::Json],
                output_modalities: vec![InferenceModality::Json],
            },
            InferenceTaskId::ImageGeneration => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::ImageGeneration,
                result_kind: InferenceExecutionResultKind::ImageGeneration,
                execution_supported: true,
                streaming_support: TaskStreamingSupport::Unsupported,
                required_input_modalities: vec![InferenceModality::Text],
                output_modalities: vec![InferenceModality::Image],
            },
            InferenceTaskId::ImageUnderstanding => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::ImageUnderstanding,
                result_kind: InferenceExecutionResultKind::ImageUnderstanding,
                execution_supported: false,
                streaming_support: TaskStreamingSupport::BackendDependent,
                required_input_modalities: vec![InferenceModality::Image, InferenceModality::Text],
                output_modalities: vec![InferenceModality::Text],
            },
            InferenceTaskId::DepthEstimation => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::DepthEstimation,
                result_kind: InferenceExecutionResultKind::DepthEstimation,
                execution_supported: false,
                streaming_support: TaskStreamingSupport::Unsupported,
                required_input_modalities: vec![InferenceModality::Image],
                output_modalities: vec![InferenceModality::Image, InferenceModality::PointCloud],
            },
            InferenceTaskId::AudioTranscription => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::AudioTranscription,
                result_kind: InferenceExecutionResultKind::AudioTranscription,
                execution_supported: true,
                streaming_support: TaskStreamingSupport::BackendDependent,
                required_input_modalities: vec![InferenceModality::Audio],
                output_modalities: vec![InferenceModality::Text],
            },
            InferenceTaskId::VideoUnderstanding => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::VideoUnderstanding,
                result_kind: InferenceExecutionResultKind::VideoUnderstanding,
                execution_supported: false,
                streaming_support: TaskStreamingSupport::Unsupported,
                required_input_modalities: vec![InferenceModality::Video, InferenceModality::Text],
                output_modalities: vec![InferenceModality::Text],
            },
            InferenceTaskId::MultimodalGeneration => Self {
                task_id: task_id.clone(),
                input_kind: InferenceExecutionInputKind::MultimodalGeneration,
                result_kind: InferenceExecutionResultKind::MultimodalGeneration,
                execution_supported: false,
                streaming_support: TaskStreamingSupport::BackendDependent,
                required_input_modalities: vec![
                    InferenceModality::Text,
                    InferenceModality::Image,
                    InferenceModality::Audio,
                ],
                output_modalities: vec![InferenceModality::Text],
            },
            InferenceTaskId::Unknown => return None,
        };
        Some(contract)
    }
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

/// Broad task family used for compatibility diagnostics without choosing a
/// runtime or scheduler policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskFamily {
    Generative,
    Embedding,
    Scoring,
    Perception,
    Multimodal,
    #[default]
    Unknown,
}

/// Execution behavior class exposed as a task fact.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskExecutionBehavior {
    Generates,
    Scores,
    ExtractsFeatures,
    ClassifiesOrDescribes,
    #[default]
    Unknown,
}

/// Whether a task can expose incremental result events.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStreamingSupport {
    Supported,
    Unsupported,
    BackendDependent,
    #[default]
    Unknown,
}

/// Registry entry for a canonical inference task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TaskRegistryEntry {
    pub task_id: InferenceTaskId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub task_family: TaskFamily,
    pub modality_signature: TaskModalitySignature,
    pub result_family: String,
    #[serde(default)]
    pub execution_behavior: TaskExecutionBehavior,
    #[serde(default)]
    pub streaming_support: TaskStreamingSupport,
    pub support_tier: SupportTier,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_components: Vec<ProcessorComponentKind>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upstream_task_ids: Vec<String>,
}

impl TaskRegistryEntry {
    /// Canonical label for this task entry.
    #[must_use]
    pub fn canonical_label(&self) -> &'static str {
        self.task_id.canonical_label()
    }

    /// Typed request/result payload contract for this registry entry.
    #[must_use]
    pub fn request_contract(&self) -> Option<TaskRequestContract> {
        TaskRequestContract::for_task(&self.task_id)
    }

    /// Return true when an input task label matches this entry's canonical id
    /// or one of its normalized aliases.
    #[must_use]
    pub fn matches_label(&self, value: &str) -> bool {
        let normalized = normalize_task_label(value);
        if normalized.is_empty() {
            return false;
        }
        normalized == self.canonical_label()
            || self
                .aliases
                .iter()
                .any(|alias| normalize_task_label(alias) == normalized)
            || self
                .upstream_task_ids
                .iter()
                .any(|alias| normalize_task_label(alias) == normalized)
    }

    /// Return true when discovered task labels match this registry entry.
    ///
    /// Empty evidence is treated as inconclusive rather than incompatible so
    /// older sparse package-fact rows can be validated by neighboring model and
    /// component facts.
    #[must_use]
    pub fn matches_task_evidence(&self, evidence: &TaskEvidence) -> bool {
        [
            evidence.task_type_primary.as_ref(),
            evidence.pipeline_tag.as_ref(),
        ]
        .into_iter()
        .flatten()
        .all(|label| self.matches_label(label))
    }

    /// Return true when discovered input/output modalities fit this registry
    /// entry's declared modality signature.
    ///
    /// Empty modality evidence is treated as inconclusive for compatibility with
    /// older package-fact rows.
    #[must_use]
    pub fn matches_modality_evidence(&self, evidence: &TaskEvidence) -> bool {
        evidence.input_modalities.iter().all(|modality| {
            self.modality_signature
                .inputs
                .iter()
                .any(|supported| normalize_modality_label(modality) == supported.canonical_label())
        }) && evidence.output_modalities.iter().all(|modality| {
            self.modality_signature
                .outputs
                .iter()
                .any(|supported| normalize_modality_label(modality) == supported.canonical_label())
        })
    }
}

/// Stable diagnostic for task-registry resolution at package/request boundaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TaskRegistryResolutionDiagnostic {
    pub kind: TaskRegistryResolutionDiagnosticKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub canonical_task_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_modalities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_modalities: Vec<String>,
}

/// Stable task-registry resolution diagnostic labels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskRegistryResolutionDiagnosticKind {
    MissingTaskEvidence,
    UnsupportedTaskLabel,
    ConflictingTaskEvidence,
    ModalityMismatch,
}

impl TaskRegistryResolutionDiagnostic {
    fn missing_task_evidence() -> Self {
        Self {
            kind: TaskRegistryResolutionDiagnosticKind::MissingTaskEvidence,
            message: "package task evidence does not include a task label".to_string(),
            labels: Vec::new(),
            canonical_task_ids: Vec::new(),
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
        }
    }

    fn unsupported_task_label(labels: Vec<String>) -> Self {
        Self {
            kind: TaskRegistryResolutionDiagnosticKind::UnsupportedTaskLabel,
            message:
                "package task evidence does not match a canonical inference task registry entry"
                    .to_string(),
            labels,
            canonical_task_ids: Vec::new(),
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
        }
    }

    fn conflicting_task_evidence(labels: Vec<String>, canonical_task_ids: Vec<String>) -> Self {
        Self {
            kind: TaskRegistryResolutionDiagnosticKind::ConflictingTaskEvidence,
            message: "package task labels resolve to different canonical inference tasks"
                .to_string(),
            labels,
            canonical_task_ids,
            input_modalities: Vec::new(),
            output_modalities: Vec::new(),
        }
    }

    fn modality_mismatch(entry: &TaskRegistryEntry, evidence: &TaskEvidence) -> Self {
        Self {
            kind: TaskRegistryResolutionDiagnosticKind::ModalityMismatch,
            message: format!(
                "package modalities do not match canonical task {}",
                entry.canonical_label()
            ),
            labels: task_evidence_labels(evidence),
            canonical_task_ids: vec![entry.canonical_label().to_string()],
            input_modalities: evidence.input_modalities.clone(),
            output_modalities: evidence.output_modalities.clone(),
        }
    }
}

/// Seeded canonical task registry entries for the first inference vertical
/// slices. This is a contract fixture, not a runtime selection table.
#[must_use]
pub fn default_task_registry_entries() -> Vec<TaskRegistryEntry> {
    vec![
        TaskRegistryEntry {
            task_id: InferenceTaskId::TextGeneration,
            aliases: vec![
                "text-generation".to_string(),
                "text_generation".to_string(),
                "generation".to_string(),
                "causal-lm".to_string(),
            ],
            task_family: TaskFamily::Generative,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Text],
                vec![InferenceModality::Text],
            ),
            result_family: "generated_text".to_string(),
            execution_behavior: TaskExecutionBehavior::Generates,
            streaming_support: TaskStreamingSupport::BackendDependent,
            support_tier: SupportTier::Stable,
            required_components: vec![ProcessorComponentKind::Tokenizer],
            upstream_task_ids: vec!["text-generation".to_string()],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::ChatCompletion,
            aliases: vec![
                "chat".to_string(),
                "chat-completion".to_string(),
                "conversational".to_string(),
            ],
            task_family: TaskFamily::Generative,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Text],
                vec![InferenceModality::Text],
            ),
            result_family: "chat_message".to_string(),
            execution_behavior: TaskExecutionBehavior::Generates,
            streaming_support: TaskStreamingSupport::BackendDependent,
            support_tier: SupportTier::Stable,
            required_components: vec![
                ProcessorComponentKind::Tokenizer,
                ProcessorComponentKind::ChatTemplate,
            ],
            upstream_task_ids: vec!["conversational".to_string()],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::Embedding,
            aliases: vec![
                "embedding".to_string(),
                "embeddings".to_string(),
                "feature-extraction".to_string(),
                "sentence-similarity".to_string(),
            ],
            task_family: TaskFamily::Embedding,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Text],
                vec![InferenceModality::Embedding],
            ),
            result_family: "embedding_vector".to_string(),
            execution_behavior: TaskExecutionBehavior::ExtractsFeatures,
            streaming_support: TaskStreamingSupport::Unsupported,
            support_tier: SupportTier::Stable,
            required_components: vec![ProcessorComponentKind::Tokenizer],
            upstream_task_ids: vec![
                "feature-extraction".to_string(),
                "sentence-similarity".to_string(),
            ],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::Rerank,
            aliases: vec![
                "rerank".to_string(),
                "reranking".to_string(),
                "text-reranking".to_string(),
            ],
            task_family: TaskFamily::Scoring,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Text, InferenceModality::Json],
                vec![InferenceModality::Json],
            ),
            result_family: "ranked_documents".to_string(),
            execution_behavior: TaskExecutionBehavior::Scores,
            streaming_support: TaskStreamingSupport::Unsupported,
            support_tier: SupportTier::Stable,
            required_components: vec![ProcessorComponentKind::Tokenizer],
            upstream_task_ids: vec!["text-ranking".to_string(), "reranking".to_string()],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::ImageGeneration,
            aliases: vec![
                "text-to-image".to_string(),
                "image-generation".to_string(),
                "image_generation".to_string(),
            ],
            task_family: TaskFamily::Generative,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Text],
                vec![InferenceModality::Image],
            ),
            result_family: "generated_image".to_string(),
            execution_behavior: TaskExecutionBehavior::Generates,
            streaming_support: TaskStreamingSupport::Unsupported,
            support_tier: SupportTier::Experimental,
            required_components: vec![ProcessorComponentKind::Processor],
            upstream_task_ids: vec!["text-to-image".to_string()],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::ImageUnderstanding,
            aliases: vec![
                "image-to-text".to_string(),
                "visual-question-answering".to_string(),
                "image-text-to-text".to_string(),
            ],
            task_family: TaskFamily::Perception,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Image, InferenceModality::Text],
                vec![InferenceModality::Text],
            ),
            result_family: "image_text".to_string(),
            execution_behavior: TaskExecutionBehavior::ClassifiesOrDescribes,
            streaming_support: TaskStreamingSupport::BackendDependent,
            support_tier: SupportTier::Experimental,
            required_components: vec![
                ProcessorComponentKind::Tokenizer,
                ProcessorComponentKind::ImageProcessor,
            ],
            upstream_task_ids: vec![
                "image-to-text".to_string(),
                "visual-question-answering".to_string(),
                "image-text-to-text".to_string(),
            ],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::DepthEstimation,
            aliases: vec![
                "depth-estimation".to_string(),
                "depth_estimation".to_string(),
                "depth".to_string(),
                "monocular-depth-estimation".to_string(),
            ],
            task_family: TaskFamily::Perception,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Image],
                vec![InferenceModality::Image, InferenceModality::PointCloud],
            ),
            result_family: "depth_map".to_string(),
            execution_behavior: TaskExecutionBehavior::ClassifiesOrDescribes,
            streaming_support: TaskStreamingSupport::Unsupported,
            support_tier: SupportTier::Roadmap,
            required_components: vec![ProcessorComponentKind::ImageProcessor],
            upstream_task_ids: vec!["depth-estimation".to_string()],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::AudioTranscription,
            aliases: vec![
                "audio-transcription".to_string(),
                "automatic-speech-recognition".to_string(),
                "speech-to-text".to_string(),
            ],
            task_family: TaskFamily::Perception,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Audio],
                vec![InferenceModality::Text],
            ),
            result_family: "transcript".to_string(),
            execution_behavior: TaskExecutionBehavior::ClassifiesOrDescribes,
            streaming_support: TaskStreamingSupport::BackendDependent,
            support_tier: SupportTier::Experimental,
            required_components: vec![ProcessorComponentKind::AudioFeatureExtractor],
            upstream_task_ids: vec!["automatic-speech-recognition".to_string()],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::VideoUnderstanding,
            aliases: vec![
                "video-to-text".to_string(),
                "video-text-to-text".to_string(),
            ],
            task_family: TaskFamily::Perception,
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Video, InferenceModality::Text],
                vec![InferenceModality::Text],
            ),
            result_family: "video_text".to_string(),
            execution_behavior: TaskExecutionBehavior::ClassifiesOrDescribes,
            streaming_support: TaskStreamingSupport::Unsupported,
            support_tier: SupportTier::Roadmap,
            required_components: vec![ProcessorComponentKind::VideoProcessor],
            upstream_task_ids: vec!["video-text-to-text".to_string()],
        },
        TaskRegistryEntry {
            task_id: InferenceTaskId::MultimodalGeneration,
            aliases: vec![
                "multimodal-generation".to_string(),
                "image-audio-text-to-text".to_string(),
            ],
            task_family: TaskFamily::Multimodal,
            modality_signature: TaskModalitySignature::new(
                vec![
                    InferenceModality::Text,
                    InferenceModality::Image,
                    InferenceModality::Audio,
                ],
                vec![InferenceModality::Text],
            ),
            result_family: "multimodal_text".to_string(),
            execution_behavior: TaskExecutionBehavior::Generates,
            streaming_support: TaskStreamingSupport::BackendDependent,
            support_tier: SupportTier::Roadmap,
            required_components: vec![
                ProcessorComponentKind::Tokenizer,
                ProcessorComponentKind::Processor,
            ],
            upstream_task_ids: vec!["image-text-to-text".to_string()],
        },
    ]
}

/// Resolve a task label or upstream alias to a seeded registry entry.
#[must_use]
pub fn resolve_task_registry_entry(value: &str) -> Option<TaskRegistryEntry> {
    default_task_registry_entries()
        .into_iter()
        .find(|entry| entry.matches_label(value))
}

/// Resolve package task evidence to a validated canonical task registry entry.
///
/// This is a boundary parser for model-library package facts. Internal backend
/// code should consume the returned `TaskRegistryEntry` rather than inspecting
/// raw task strings directly.
///
/// # Errors
///
/// Returns a typed diagnostic when evidence is missing, unsupported, resolves
/// to conflicting canonical tasks, or declares modalities outside the resolved
/// task signature.
pub fn resolve_task_registry_entry_from_evidence(
    evidence: &TaskEvidence,
) -> Result<TaskRegistryEntry, TaskRegistryResolutionDiagnostic> {
    let labels = task_evidence_labels(evidence);
    if labels.is_empty() {
        return Err(TaskRegistryResolutionDiagnostic::missing_task_evidence());
    }

    let mut resolved_entries = Vec::new();
    for label in &labels {
        if let Some(entry) = resolve_task_registry_entry(label) {
            resolved_entries.push(entry);
        }
    }

    let Some(first) = resolved_entries.first().cloned() else {
        return Err(TaskRegistryResolutionDiagnostic::unsupported_task_label(
            labels,
        ));
    };

    let mut canonical_task_ids = vec![first.canonical_label().to_string()];
    for entry in resolved_entries.iter().skip(1) {
        if entry.task_id != first.task_id {
            let label = entry.canonical_label().to_string();
            if !canonical_task_ids.contains(&label) {
                canonical_task_ids.push(label);
            }
        }
    }

    if canonical_task_ids.len() > 1 {
        return Err(TaskRegistryResolutionDiagnostic::conflicting_task_evidence(
            labels,
            canonical_task_ids,
        ));
    }

    if !first.matches_task_evidence(evidence) {
        return Err(TaskRegistryResolutionDiagnostic::unsupported_task_label(
            labels,
        ));
    }

    if !first.matches_modality_evidence(evidence) {
        return Err(TaskRegistryResolutionDiagnostic::modality_mismatch(
            &first, evidence,
        ));
    }

    Ok(first)
}

fn task_evidence_labels(evidence: &TaskEvidence) -> Vec<String> {
    [
        evidence.task_type_primary.as_deref(),
        evidence.pipeline_tag.as_deref(),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .filter(|label| !label.is_empty())
    .map(ToOwned::to_owned)
    .collect()
}

/// Normalize a task label for registry alias matching.
#[must_use]
pub fn normalize_task_label(value: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('_');
            last_was_separator = true;
        }
    }
    normalized.trim_matches('_').to_string()
}

/// Normalize a modality label for registry evidence matching.
#[must_use]
pub fn normalize_modality_label(value: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('_');
            last_was_separator = true;
        }
    }
    normalized.trim_matches('_').to_string()
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

/// Source strength for a package fact value.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageFactValueSource {
    Header,
    Config,
    UpstreamMetadata,
    ComponentLayout,
    FilenameWeak,
    Ambiguous,
    Unavailable,
}

/// Image-generation family labels produced only from package evidence.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageGenerationFamilyLabel {
    StableDiffusion,
    StableDiffusionXl,
    Flux,
    Flux2,
    QwenImage,
    LuminaImage,
    GlmImage,
    ZImage,
    Unknown,
    Ambiguous,
}

/// Package evidence source used for image-generation family labels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageGenerationFamilyEvidenceSource {
    PipelineClass,
    ModelIndexComponent,
    ComponentConfig,
    RepoMetadata,
    Ambiguous,
}

/// Source-tagged image-generation family evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ImageGenerationFamilyEvidence {
    pub family: ImageGenerationFamilyLabel,
    pub source: ImageGenerationFamilyEvidenceSource,
    pub value_source: PackageFactValueSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Stable roles for Diffusers-style package components.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiffusersComponentRole {
    PipelineIndex,
    Scheduler,
    Tokenizer,
    Tokenizer2,
    TextEncoder,
    TextEncoder2,
    TextEncoder3,
    ImageProcessor,
    Processor,
    Unet,
    Transformer,
    Vae,
    Controlnet,
    Adapter,
    Weights,
    GenerationConfig,
}

/// Component facts from a Diffusers bundle without importing Python classes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DiffusersComponentFacts {
    pub role: DiffusersComponentRole,
    pub status: PackageFactStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_library: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_model_type: Option<String>,
}

/// Diffusers bundle evidence from `model_index.json` and bounded component configs.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DiffusersPackageEvidence {
    pub status: PackageFactStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pipeline_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diffusers_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_or_path: Option<String>,
    pub task: TaskEvidence,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub family_evidence: Vec<ImageGenerationFamilyEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<DiffusersComponentFacts>,
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

/// Explicit remote-code trust decision for model loading.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelRemoteCodePolicy {
    #[default]
    Deny,
    Allow,
}

/// Whether model loading may contact a remote registry or must use local files.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelLoadNetworkPolicy {
    #[default]
    LocalOnly,
    AllowNetwork,
}

/// Cache policy requested at a model-loading boundary.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelLoadCachePolicy {
    #[default]
    BackendDefault,
    UseCache,
    BypassCache,
}

/// Source class for model-registry authentication, never the token itself.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelAuthTokenSource {
    #[default]
    None,
    Environment,
    HostProvided,
    HuggingFaceCache,
}

/// Stable security policy for loading model packages.
///
/// This contract is owned by Rust and can be mapped to Transformers/PyTorch,
/// vLLM, MLX, Candle, or llama.cpp adapters. It records policy decisions and
/// token source classes without carrying secret token values.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ModelLoadSecurityPolicy {
    #[serde(default)]
    pub trust_remote_code: ModelRemoteCodePolicy,
    #[serde(default)]
    pub network: ModelLoadNetworkPolicy,
    #[serde(default)]
    pub cache: ModelLoadCachePolicy,
    #[serde(default)]
    pub auth_token_source: ModelAuthTokenSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_code_sources: Vec<String>,
}

impl ModelLoadSecurityPolicy {
    #[must_use]
    pub fn allow_remote_code(&self) -> bool {
        self.trust_remote_code == ModelRemoteCodePolicy::Allow
    }

    #[must_use]
    pub fn local_files_only(&self) -> bool {
        self.network == ModelLoadNetworkPolicy::LocalOnly
    }
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
    /// Backend-local escape hatch for adapter-specific options.
    ///
    /// Public callers should prefer the typed option groups above. Extension
    /// keys must be scoped as `<backend-or-adapter>:<option>` so backend
    /// adapters can reject foreign scopes without accepting raw kwargs as
    /// stable Pantograph contract fields.
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

    /// Return diagnostics for backend-extension keys that are not scoped.
    ///
    /// Extension keys are intentionally backend-local, but the scope itself is
    /// part of the stable Pantograph wire contract so adapters can reject
    /// foreign or malformed options explicitly.
    #[must_use]
    pub fn backend_extension_scope_diagnostics(&self) -> Vec<OptionCompatibilityDiagnostic> {
        self.backend_extensions
            .keys()
            .filter(|key| !backend_extension_key_is_scoped(key))
            .map(|key| OptionCompatibilityDiagnostic {
                option_path: format!("backend_extensions.{key}"),
                state: OptionSupportState::Rejected,
                backend_key: None,
                message: Some(
                    "backend extension keys must be scoped as <backend-or-adapter>:<option>"
                        .to_string(),
                ),
            })
            .collect()
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

fn backend_extension_key_is_scoped(key: &str) -> bool {
    key.split_once(':')
        .is_some_and(|(scope, option)| !scope.trim().is_empty() && !option.trim().is_empty())
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

/// Source kind for a backend-loadable model artifact.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolvedModelSourceKind {
    /// Source was resolved through Pumas and carries a stable model reference.
    PumasResolved,
    /// Direct local Hugging Face-compatible package directory for import/debug.
    DirectHfCompatibleDirectory,
    /// Direct Hugging Face repository id for backend-local download/cache use.
    HuggingFaceRepo,
    /// Direct local GGUF path for import/debug.
    DirectGgufPath,
    /// Direct local safetensors file for import/debug.
    DirectSafetensorsPath,
    /// Direct local diffusers bundle directory for import/debug.
    DirectDiffusersBundle,
    /// Direct local ONNX model path for import/debug.
    DirectOnnxPath,
    #[default]
    Unknown,
}

/// Stable model source contract consumed by backend adapters.
///
/// This describes what should be loaded, where it came from, and what artifact
/// class it represents. It does not choose a backend, scheduler policy, runtime
/// residency, or admission behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedModelSource {
    pub source_contract_version: u32,
    pub source_kind: ResolvedModelSourceKind,
    pub artifact_kind: ModelArtifactKind,
    pub entry_path: String,
    pub storage_kind: ModelStorageKind,
    pub validation_state: ModelValidationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_ref: Option<PumasModelRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub companion_artifacts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ModelPackageDiagnostic>,
}

impl ResolvedModelSource {
    /// Build a backend-loadable source from canonical Pumas package facts.
    #[must_use]
    pub fn from_package_facts(facts: &ResolvedModelPackageFacts) -> Self {
        Self {
            source_contract_version: MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
            source_kind: ResolvedModelSourceKind::PumasResolved,
            artifact_kind: facts.artifact.artifact_kind.clone(),
            entry_path: facts.artifact.entry_path.clone(),
            storage_kind: facts.artifact.storage_kind.clone(),
            validation_state: facts.artifact.validation_state.clone(),
            model_ref: Some(facts.model_ref.clone()),
            repo_id: facts
                .transformers
                .as_ref()
                .and_then(|evidence| evidence.source_repo_id.clone()),
            revision: facts
                .transformers
                .as_ref()
                .and_then(|evidence| evidence.source_revision.clone())
                .or_else(|| facts.model_ref.revision.clone()),
            selected_files: facts.artifact.selected_files.clone(),
            companion_artifacts: facts.artifact.companion_artifacts.clone(),
            diagnostics: facts.diagnostics.clone(),
        }
    }

    /// Build a direct local source for debug/import compatibility paths.
    #[must_use]
    pub fn direct_local(
        source_kind: ResolvedModelSourceKind,
        artifact_kind: ModelArtifactKind,
        entry_path: impl Into<String>,
    ) -> Self {
        Self {
            source_contract_version: MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
            source_kind,
            artifact_kind,
            entry_path: entry_path.into(),
            storage_kind: ModelStorageKind::ExternalReference,
            validation_state: ModelValidationState::Unknown,
            model_ref: None,
            repo_id: None,
            revision: None,
            selected_files: Vec::new(),
            companion_artifacts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Build a Hugging Face repo source for backend-local download/cache use.
    #[must_use]
    pub fn hugging_face_repo(
        repo_id: impl Into<String>,
        revision: Option<String>,
        artifact_kind: ModelArtifactKind,
    ) -> Self {
        let repo_id = repo_id.into();
        Self {
            source_contract_version: MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
            source_kind: ResolvedModelSourceKind::HuggingFaceRepo,
            artifact_kind,
            entry_path: repo_id.clone(),
            storage_kind: ModelStorageKind::ExternalReference,
            validation_state: ModelValidationState::Unknown,
            model_ref: None,
            repo_id: Some(repo_id),
            revision,
            selected_files: Vec::new(),
            companion_artifacts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// True when this source came through the Pumas model identity boundary.
    #[must_use]
    pub fn is_pumas_resolved(&self) -> bool {
        self.source_kind == ResolvedModelSourceKind::PumasResolved && self.model_ref.is_some()
    }

    /// Validate the source before it is handed to a backend adapter.
    ///
    /// This enforces model-source shape invariants without selecting a backend
    /// or deciding runtime placement.
    #[must_use]
    pub fn validate_for_backend_load(&self) -> Result<(), Vec<ModelPackageDiagnostic>> {
        let mut diagnostics = Vec::new();

        if self.source_contract_version != MODEL_PACKAGE_FACTS_CONTRACT_VERSION {
            diagnostics.push(ModelPackageDiagnostic {
                code: "model_source_contract_version_mismatch".to_string(),
                message: format!(
                    "resolved model source contract version {} does not match supported version {}",
                    self.source_contract_version, MODEL_PACKAGE_FACTS_CONTRACT_VERSION
                ),
                path: Some("source_contract_version".to_string()),
            });
        }

        if self.entry_path.trim().is_empty() {
            diagnostics.push(ModelPackageDiagnostic {
                code: "model_source_missing_entry_path".to_string(),
                message: "resolved model source must include a non-empty entry path".to_string(),
                path: Some("entry_path".to_string()),
            });
        }

        if self.source_kind == ResolvedModelSourceKind::PumasResolved && self.model_ref.is_none() {
            diagnostics.push(ModelPackageDiagnostic {
                code: "pumas_resolved_source_missing_model_ref".to_string(),
                message: "Pumas-resolved model sources must carry the canonical model reference"
                    .to_string(),
                path: Some("model_ref".to_string()),
            });
        }

        if self.source_kind != ResolvedModelSourceKind::PumasResolved && self.model_ref.is_some() {
            diagnostics.push(ModelPackageDiagnostic {
                code: "direct_source_has_pumas_model_ref".to_string(),
                message: "direct model sources must not carry a Pumas model reference".to_string(),
                path: Some("model_ref".to_string()),
            });
        }

        if self.source_kind == ResolvedModelSourceKind::HuggingFaceRepo
            && self
                .repo_id
                .as_deref()
                .is_none_or(|repo_id| repo_id.trim().is_empty())
        {
            diagnostics.push(ModelPackageDiagnostic {
                code: "hugging_face_repo_source_missing_repo_id".to_string(),
                message: "Hugging Face repository sources must carry a repository id".to_string(),
                path: Some("repo_id".to_string()),
            });
        }

        if self.source_kind == ResolvedModelSourceKind::Unknown {
            diagnostics.push(ModelPackageDiagnostic {
                code: "model_source_kind_unknown".to_string(),
                message: "resolved model source kind must be known before backend loading"
                    .to_string(),
                path: Some("source_kind".to_string()),
            });
        }

        if self.artifact_kind == ModelArtifactKind::Unknown {
            diagnostics.push(ModelPackageDiagnostic {
                code: "model_source_artifact_kind_unknown".to_string(),
                message: "resolved model source artifact kind must be known before backend loading"
                    .to_string(),
                path: Some("artifact_kind".to_string()),
            });
        }

        if self.validation_state == ModelValidationState::Invalid {
            diagnostics.push(ModelPackageDiagnostic {
                code: "model_source_artifact_invalid".to_string(),
                message: "invalid model artifacts cannot be handed to backend loading".to_string(),
                path: Some("validation_state".to_string()),
            });
        }

        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(diagnostics)
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diffusers: Option<DiffusersPackageEvidence>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_contract(task_id: InferenceTaskId) -> TaskRequestContract {
        resolve_task_registry_entry(task_id.canonical_label())
            .and_then(|entry| entry.request_contract())
            .unwrap_or_else(|| panic!("missing request contract for {task_id:?}"))
    }

    #[test]
    fn task_registry_entries_publish_typed_request_contracts() {
        let text = registry_contract(InferenceTaskId::TextGeneration);
        assert_eq!(text.input_kind, InferenceExecutionInputKind::TextGeneration);
        assert_eq!(
            text.result_kind,
            InferenceExecutionResultKind::TextGeneration
        );
        assert!(text.execution_supported);

        let chat = registry_contract(InferenceTaskId::ChatCompletion);
        assert_eq!(chat.input_kind, InferenceExecutionInputKind::TextGeneration);
        assert_eq!(
            chat.result_kind,
            InferenceExecutionResultKind::TextGeneration
        );
        assert!(chat.execution_supported);

        let embedding = registry_contract(InferenceTaskId::Embedding);
        assert_eq!(embedding.input_kind, InferenceExecutionInputKind::Embedding);
        assert_eq!(
            embedding.result_kind,
            InferenceExecutionResultKind::Embedding
        );
        assert!(embedding.execution_supported);

        let rerank = registry_contract(InferenceTaskId::Rerank);
        assert_eq!(rerank.input_kind, InferenceExecutionInputKind::Rerank);
        assert_eq!(rerank.result_kind, InferenceExecutionResultKind::Rerank);
        assert!(rerank.execution_supported);

        let image = registry_contract(InferenceTaskId::ImageGeneration);
        assert_eq!(
            image.input_kind,
            InferenceExecutionInputKind::ImageGeneration
        );
        assert_eq!(
            image.result_kind,
            InferenceExecutionResultKind::ImageGeneration
        );
        assert!(image.execution_supported);

        let depth = registry_contract(InferenceTaskId::DepthEstimation);
        assert_eq!(
            depth.input_kind,
            InferenceExecutionInputKind::DepthEstimation
        );
        assert_eq!(
            depth.result_kind,
            InferenceExecutionResultKind::DepthEstimation
        );
        assert_eq!(
            depth.required_input_modalities,
            vec![InferenceModality::Image]
        );
        assert_eq!(
            depth.output_modalities,
            vec![InferenceModality::Image, InferenceModality::PointCloud]
        );
        assert!(!depth.execution_supported);
    }

    #[test]
    fn audio_transcription_task_contract_is_executable() {
        let audio = registry_contract(InferenceTaskId::AudioTranscription);

        assert_eq!(
            audio.input_kind,
            InferenceExecutionInputKind::AudioTranscription
        );
        assert_eq!(
            audio.result_kind,
            InferenceExecutionResultKind::AudioTranscription
        );
        assert!(audio.execution_supported);
    }

    #[test]
    fn roadmap_task_request_contracts_are_not_executable() {
        let image_understanding = registry_contract(InferenceTaskId::ImageUnderstanding);
        assert_eq!(
            image_understanding.input_kind,
            InferenceExecutionInputKind::ImageUnderstanding
        );
        assert_eq!(
            image_understanding.result_kind,
            InferenceExecutionResultKind::ImageUnderstanding
        );
        assert_eq!(
            image_understanding.streaming_support,
            TaskStreamingSupport::BackendDependent
        );
        assert!(!image_understanding.execution_supported);

        let video_understanding = registry_contract(InferenceTaskId::VideoUnderstanding);
        assert_eq!(
            video_understanding.input_kind,
            InferenceExecutionInputKind::VideoUnderstanding
        );
        assert_eq!(
            video_understanding.result_kind,
            InferenceExecutionResultKind::VideoUnderstanding
        );
        assert_eq!(
            video_understanding.streaming_support,
            TaskStreamingSupport::Unsupported
        );
        assert!(!video_understanding.execution_supported);

        let multimodal_generation = registry_contract(InferenceTaskId::MultimodalGeneration);
        assert_eq!(
            multimodal_generation.input_kind,
            InferenceExecutionInputKind::MultimodalGeneration
        );
        assert_eq!(
            multimodal_generation.result_kind,
            InferenceExecutionResultKind::MultimodalGeneration
        );
        assert_eq!(
            multimodal_generation.streaming_support,
            TaskStreamingSupport::BackendDependent
        );
        assert!(!multimodal_generation.execution_supported);
    }

    #[test]
    fn task_request_contract_serde_uses_stable_snake_case() {
        let contract = registry_contract(InferenceTaskId::ImageGeneration);
        let encoded = serde_json::to_value(&contract).unwrap();
        let decoded: TaskRequestContract = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["task_id"], serde_json::json!("image_generation"));
        assert_eq!(encoded["input_kind"], serde_json::json!("image_generation"));
        assert_eq!(
            encoded["result_kind"],
            serde_json::json!("image_generation")
        );
        assert_eq!(decoded, contract);
    }

    #[test]
    fn task_request_contract_serde_defaults_and_unknown_fields_are_additive() {
        let encoded = serde_json::json!({
            "task_id": "embedding",
            "input_kind": "embedding",
            "result_kind": "embedding",
            "execution_supported": true,
            "streaming_support": "unsupported",
            "future_registry_field": "ignored_by_current_consumer"
        });

        let decoded: TaskRequestContract = serde_json::from_value(encoded).unwrap();

        assert_eq!(decoded.task_id, InferenceTaskId::Embedding);
        assert_eq!(decoded.required_input_modalities, Vec::new());
        assert_eq!(decoded.output_modalities, Vec::new());
    }

    #[test]
    fn task_request_contracts_match_registry_modalities_and_streaming() {
        for entry in default_task_registry_entries() {
            let contract = entry
                .request_contract()
                .unwrap_or_else(|| panic!("missing request contract for {:?}", entry.task_id));
            assert_eq!(contract.task_id, entry.task_id);
            assert_eq!(contract.streaming_support, entry.streaming_support);
            assert_eq!(
                contract.required_input_modalities,
                entry.modality_signature.inputs
            );
            assert_eq!(contract.output_modalities, entry.modality_signature.outputs);
        }
    }

    #[test]
    fn generation_options_separate_stable_fields_from_backend_extensions() {
        let options = GenerationOptions {
            length: LengthGenerationOptions {
                max_new_tokens: Some(128),
                ..Default::default()
            },
            sampling: SamplingGenerationOptions {
                temperature: Some(0.4),
                ..Default::default()
            },
            backend_extensions: [
                (
                    "transformers:renormalize_logits".to_string(),
                    serde_json::json!(true),
                ),
                ("llama.cpp:mirostat".to_string(), serde_json::json!(2)),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        let requested = options.requested_option_paths();

        assert!(requested.contains(&"length.max_new_tokens".to_string()));
        assert!(requested.contains(&"sampling.temperature".to_string()));
        assert!(
            requested.contains(&"backend_extensions.transformers:renormalize_logits".to_string())
        );
        assert!(requested.contains(&"backend_extensions.llama.cpp:mirostat".to_string()));
    }

    #[test]
    fn generation_options_wire_shape_is_additive_and_defaults_missing_groups() {
        let encoded = serde_json::json!({
            "length": {
                "max_new_tokens": 32
            },
            "backend_extensions": {
                "transformers:renormalize_logits": true
            },
            "future_generation_group": {
                "speculative_decoding": true
            }
        });

        let decoded: GenerationOptions = serde_json::from_value(encoded).unwrap();

        assert_eq!(decoded.length.max_new_tokens, Some(32));
        assert_eq!(decoded.sampling.temperature, None);
        assert_eq!(decoded.search.num_beams, None);
        assert_eq!(
            decoded.backend_extensions["transformers:renormalize_logits"],
            serde_json::json!(true)
        );
    }
}
