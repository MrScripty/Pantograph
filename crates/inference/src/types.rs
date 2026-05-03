//! Common types for inference operations

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model_contracts::{
    GenerationOptions, InferenceLifecyclePhase, InferenceTaskId, OptionCompatibilityDiagnostic,
    PumasModelRef,
};

/// Chat message with multimodal content support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: String,
    pub content: Vec<ContentPart>,
}

/// Content part - text or image
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlData },
}

/// Image URL data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageUrlData {
    pub url: String,
}

/// Chat completion request (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// Canonical task execution request consumed by future typed backend paths.
///
/// This DTO is independent of OpenAI-compatible transport JSON. Backend
/// adapters may still translate it to OpenAI, Transformers, llama.cpp, vLLM, or
/// other native request shapes internally.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct InferenceExecutionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub task_id: InferenceTaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_ref: Option<PumasModelRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_hint: Option<String>,
    pub input: InferenceExecutionInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_options: Option<GenerationOptions>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

impl InferenceExecutionRequest {
    /// Convert an OpenAI-compatible chat request at the adapter edge.
    ///
    /// The returned request still needs normal validation through
    /// [`InferenceExecutionRequest::validate`] before execution.
    #[must_use]
    pub fn from_openai_chat_request(request_id: Option<String>, request: ChatRequest) -> Self {
        let generation_options = if request.max_tokens.is_some() || request.temperature.is_some() {
            Some(GenerationOptions {
                length: crate::model_contracts::LengthGenerationOptions {
                    max_new_tokens: request.max_tokens,
                    ..Default::default()
                },
                sampling: crate::model_contracts::SamplingGenerationOptions {
                    temperature: request.temperature,
                    ..Default::default()
                },
                ..Default::default()
            })
        } else {
            None
        };

        Self {
            request_id,
            task_id: InferenceTaskId::ChatCompletion,
            model_ref: None,
            model_name: Some(request.model),
            runtime_hint: None,
            input: InferenceExecutionInput::TextGeneration {
                prompt: None,
                system_prompt: None,
                messages: request.messages,
                stream: request.stream,
            },
            generation_options,
            extra_options: Value::Null,
        }
    }

    /// Validate task/input consistency before backend execution.
    ///
    /// # Errors
    ///
    /// Returns [`InferenceExecutionRequestValidationError`] when the task id and
    /// input variant disagree, when required text/query/document payloads are
    /// empty, or when the task is not supported by the typed request contract.
    pub fn validate(&self) -> Result<(), InferenceExecutionRequestValidationError> {
        match (&self.task_id, &self.input) {
            (
                InferenceTaskId::TextGeneration | InferenceTaskId::ChatCompletion,
                InferenceExecutionInput::TextGeneration {
                    prompt, messages, ..
                },
            ) => {
                if prompt.as_deref().is_none_or(str::is_empty) && messages.is_empty() {
                    return Err(InferenceExecutionRequestValidationError::MissingTextInput);
                }
                Ok(())
            }
            (InferenceTaskId::Embedding, InferenceExecutionInput::Embedding { texts }) => {
                if texts.is_empty() {
                    return Err(InferenceExecutionRequestValidationError::EmptyEmbeddingTexts);
                }
                Ok(())
            }
            (
                InferenceTaskId::Rerank,
                InferenceExecutionInput::Rerank {
                    query, documents, ..
                },
            ) => {
                if query.is_empty() {
                    return Err(InferenceExecutionRequestValidationError::EmptyRerankQuery);
                }
                if documents.is_empty() {
                    return Err(InferenceExecutionRequestValidationError::EmptyRerankDocuments);
                }
                Ok(())
            }
            (InferenceTaskId::ImageGeneration, InferenceExecutionInput::ImageGeneration { .. }) => {
                Ok(())
            }
            (
                InferenceTaskId::Unknown
                | InferenceTaskId::ImageUnderstanding
                | InferenceTaskId::AudioTranscription
                | InferenceTaskId::VideoUnderstanding
                | InferenceTaskId::MultimodalGeneration,
                _,
            ) => Err(InferenceExecutionRequestValidationError::UnsupportedTask {
                task_id: self.task_id.clone(),
            }),
            _ => Err(
                InferenceExecutionRequestValidationError::TaskInputMismatch {
                    task_id: self.task_id.clone(),
                    input_type: self.input.input_type_label(),
                },
            ),
        }
    }
}

/// Typed request validation failure at the inference execution boundary.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InferenceExecutionRequestValidationError {
    #[error("text generation requires a prompt or chat messages")]
    MissingTextInput,
    #[error("embedding execution requires at least one text input")]
    EmptyEmbeddingTexts,
    #[error("rerank execution requires a query")]
    EmptyRerankQuery,
    #[error("rerank execution requires at least one document")]
    EmptyRerankDocuments,
    #[error("task {task_id:?} does not match input type {input_type}")]
    TaskInputMismatch {
        task_id: InferenceTaskId,
        input_type: &'static str,
    },
    #[error("task {task_id:?} is not supported by the typed execution request contract")]
    UnsupportedTask { task_id: InferenceTaskId },
}

/// Canonical task input payloads, separated from backend transport formats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "input_type", rename_all = "snake_case")]
pub enum InferenceExecutionInput {
    TextGeneration {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        system_prompt: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        messages: Vec<ChatMessage>,
        #[serde(default)]
        stream: bool,
    },
    Embedding {
        texts: Vec<String>,
    },
    Rerank {
        query: String,
        documents: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        top_n: Option<usize>,
        #[serde(default)]
        return_documents: bool,
    },
    ImageGeneration {
        request: ImageGenerationRequest,
    },
}

impl InferenceExecutionInput {
    #[must_use]
    pub fn input_type_label(&self) -> &'static str {
        match self {
            Self::TextGeneration { .. } => "text_generation",
            Self::Embedding { .. } => "embedding",
            Self::Rerank { .. } => "rerank",
            Self::ImageGeneration { .. } => "image_generation",
        }
    }
}

/// Canonical task execution result emitted by future typed backend paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "result_type", rename_all = "snake_case")]
pub enum InferenceExecutionResult {
    TextGeneration {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<InferenceUsage>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_handle_id: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
    Embedding {
        embeddings: Vec<InferenceEmbeddingResult>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<InferenceUsage>,
    },
    Rerank {
        response: RerankResponse,
    },
    ImageGeneration {
        result: ImageGenerationResult,
    },
}

/// Token or item usage attached to a typed execution result when available.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct InferenceUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

/// Typed embedding item for canonical execution results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct InferenceEmbeddingResult {
    pub vector: Vec<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
}

/// Base64-encoded image payload used across image-generation requests/results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedImage {
    /// Base64-encoded image bytes.
    pub data_base64: String,
    /// MIME type describing the encoded image payload.
    pub mime_type: String,
    /// Optional image width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Optional image height in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// Text-to-image request contract used by diffusion-capable backends.
///
/// The request is append-only by design so later modes (img2img, inpaint)
/// can reuse the same contract with optional `init_image` / `mask_image`
/// fields instead of replacing it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageGenerationRequest {
    /// Backend-specific model identifier or path.
    pub model: String,
    /// Positive prompt describing the desired image.
    pub prompt: String,
    /// Optional negative prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    /// Target image width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    /// Target image height in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Number of denoising steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_inference_steps: Option<u32>,
    /// Guidance / CFG scale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f32>,
    /// Deterministic seed, if supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    /// Optional scheduler identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduler: Option<String>,
    /// Number of images to produce for the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_images_per_prompt: Option<u32>,
    /// Optional init image reserved for later img2img support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_image: Option<EncodedImage>,
    /// Optional mask image reserved for later inpaint support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask_image: Option<EncodedImage>,
    /// Optional denoise strength reserved for later img2img/inpaint support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<f32>,
    /// Backend/model-specific append-only options.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

/// Image-generation response contract returned by diffusion-capable backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageGenerationResult {
    /// Generated image payloads.
    pub images: Vec<EncodedImage>,
    /// Effective seed used by the backend, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_used: Option<u64>,
    /// Optional backend metadata such as scheduler or timings.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

/// Reranking request contract shared across backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerankRequest {
    /// Backend-specific model identifier or path.
    pub model: String,
    /// Query to rank documents against.
    pub query: String,
    /// Candidate documents in original input order.
    pub documents: Vec<String>,
    /// Optional top-N truncation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_n: Option<usize>,
    /// Whether to include document text in results.
    #[serde(default)]
    pub return_documents: bool,
    /// Backend/model-specific append-only options.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

/// A single reranked result item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerankResult {
    /// Original candidate index.
    pub index: usize,
    /// Normalized relevance score.
    pub score: f32,
    /// Optional document text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
}

/// Reranking response contract returned by reranker-capable backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RerankResponse {
    /// Ranked results in output order.
    pub results: Vec<RerankResult>,
    /// Optional backend metadata such as timings.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

/// Streaming response chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub choices: Vec<StreamChoice>,
}

/// Streaming choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

/// Delta content in streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
}

/// Server event for streaming
#[derive(Clone, Serialize)]
pub struct StreamEvent {
    pub content: Option<String>,
    pub done: bool,
    pub error: Option<String>,
}

/// Request-scoped lifecycle event kind emitted by diagnostics-aware inference
/// callers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InferenceRequestLifecycleEventKind {
    Started,
    Completed,
    Failed,
    Cancelled,
    CleanupCompleted,
}

/// Request-scoped inference lifecycle event.
///
/// Events are facts for diagnostics and auditing. They do not control runtime
/// selection, scheduling, or backend execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct InferenceRequestLifecycleEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub phase: InferenceLifecyclePhase,
    pub kind: InferenceRequestLifecycleEventKind,
    pub occurred_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Synchronous sink for request lifecycle facts.
pub trait InferenceRequestLifecycleEventSink: Send + Sync {
    fn record(&self, event: InferenceRequestLifecycleEvent);
}

/// Snapshot of an inference runtime lifecycle owned by the backend.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeLifecycleSnapshot {
    #[serde(default)]
    pub runtime_id: Option<String>,
    #[serde(default)]
    pub runtime_instance_id: Option<String>,
    #[serde(default)]
    pub warmup_started_at_ms: Option<u64>,
    #[serde(default)]
    pub warmup_completed_at_ms: Option<u64>,
    #[serde(default)]
    pub warmup_duration_ms: Option<u64>,
    #[serde(default)]
    pub runtime_reused: Option<bool>,
    #[serde(default)]
    pub lifecycle_decision_reason: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub last_error: Option<String>,
}

impl RuntimeLifecycleSnapshot {
    pub fn runtime_fact_readiness(&self) -> RuntimeFactReadiness {
        if self.active {
            if self.warmup_started_at_ms.is_some() && self.warmup_completed_at_ms.is_none() {
                return RuntimeFactReadiness::Warming;
            }

            return RuntimeFactReadiness::Ready;
        }

        if self.last_error.is_some() {
            return RuntimeFactReadiness::Failed;
        }

        RuntimeFactReadiness::Stopped
    }

    pub fn runtime_fact_reuse_result(&self) -> RuntimeFactReuseResult {
        match self.runtime_reused {
            Some(true) => RuntimeFactReuseResult::Reused,
            Some(false) => RuntimeFactReuseResult::Started,
            None => RuntimeFactReuseResult::Unknown,
        }
    }

    pub fn default_lifecycle_decision_reason(&self) -> Option<&'static str> {
        match (self.last_error.as_ref(), self.runtime_reused, self.active) {
            (Some(_), _, _) => Some("runtime_start_failed"),
            (None, Some(true), true) => Some("runtime_reused"),
            (None, _, true) => Some("runtime_ready"),
            (None, _, false) => None,
        }
    }

    pub fn normalized_lifecycle_decision_reason(&self) -> Option<String> {
        self.lifecycle_decision_reason
            .clone()
            .or_else(|| self.default_lifecycle_decision_reason().map(str::to_string))
    }
}

/// Normalized runtime readiness fact reported by inference.
///
/// This is an observed backend lifecycle state, not a scheduler admission or
/// runtime-selection conclusion.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFactReadiness {
    Ready,
    Warming,
    Failed,
    Stopped,
    Unsupported,
    Unknown,
}

/// Normalized reuse result for the runtime that produced a fact snapshot.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFactReuseResult {
    Reused,
    Started,
    NotApplicable,
    Unknown,
}

/// Reason inference has no loaded runtime fact for a backend/runtime slot.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFactAbsenceReason {
    Unloaded,
    Failed,
    Unsupported,
}

/// Inference-owned runtime fact snapshot exposed to host layers.
///
/// The DTO normalizes raw lifecycle fields into stable readiness/reuse/absence
/// facts while keeping scheduler policy and runtime selection outside the
/// inference crate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeFactSnapshot {
    #[serde(default)]
    pub backend_key: Option<String>,
    #[serde(default)]
    pub runtime_id: Option<String>,
    #[serde(default)]
    pub runtime_instance_id: Option<String>,
    #[serde(default)]
    pub active_model_target: Option<String>,
    #[serde(default)]
    pub resolved_device: Option<String>,
    #[serde(default)]
    pub warmup_started_at_ms: Option<u64>,
    #[serde(default)]
    pub warmup_completed_at_ms: Option<u64>,
    #[serde(default)]
    pub warmup_duration_ms: Option<u64>,
    pub reuse_result: RuntimeFactReuseResult,
    pub readiness: RuntimeFactReadiness,
    #[serde(default)]
    pub absence_reason: Option<RuntimeFactAbsenceReason>,
    #[serde(default)]
    pub lifecycle_decision_reason: Option<String>,
    #[serde(default)]
    pub last_backend_error: Option<String>,
}

impl RuntimeFactSnapshot {
    pub fn from_lifecycle(
        backend_key: Option<String>,
        active_model_target: Option<String>,
        resolved_device: Option<String>,
        lifecycle: RuntimeLifecycleSnapshot,
    ) -> Self {
        let readiness = lifecycle.runtime_fact_readiness();
        let reuse_result = lifecycle.runtime_fact_reuse_result();
        let lifecycle_decision_reason = lifecycle.lifecycle_decision_reason.clone().or_else(|| {
            if readiness == RuntimeFactReadiness::Warming {
                Some("runtime_warming".to_string())
            } else {
                lifecycle
                    .default_lifecycle_decision_reason()
                    .map(str::to_string)
            }
        });
        let absence_reason = match readiness {
            RuntimeFactReadiness::Failed if !lifecycle.active => {
                Some(RuntimeFactAbsenceReason::Failed)
            }
            _ => None,
        };

        Self {
            backend_key,
            runtime_id: lifecycle.runtime_id,
            runtime_instance_id: lifecycle.runtime_instance_id,
            active_model_target,
            resolved_device,
            warmup_started_at_ms: lifecycle.warmup_started_at_ms,
            warmup_completed_at_ms: lifecycle.warmup_completed_at_ms,
            warmup_duration_ms: lifecycle.warmup_duration_ms,
            reuse_result,
            readiness,
            absence_reason,
            lifecycle_decision_reason,
            last_backend_error: lifecycle.last_error,
        }
    }

    pub fn absent_backend(
        backend_key: Option<String>,
        absence_reason: RuntimeFactAbsenceReason,
    ) -> Self {
        let readiness = match absence_reason {
            RuntimeFactAbsenceReason::Unloaded => RuntimeFactReadiness::Stopped,
            RuntimeFactAbsenceReason::Failed => RuntimeFactReadiness::Failed,
            RuntimeFactAbsenceReason::Unsupported => RuntimeFactReadiness::Unsupported,
        };

        Self {
            backend_key,
            runtime_id: None,
            runtime_instance_id: None,
            active_model_target: None,
            resolved_device: None,
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            reuse_result: RuntimeFactReuseResult::NotApplicable,
            readiness,
            absence_reason: Some(absence_reason),
            lifecycle_decision_reason: None,
            last_backend_error: None,
        }
    }
}

/// Server operating mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerModeInfo {
    /// Backend identifier that owns the active runtime selection.
    #[serde(default)]
    pub backend_name: Option<String>,
    /// Stable backend key for automation and UI selection state.
    #[serde(default)]
    pub backend_key: Option<String>,
    /// Current mode type
    pub mode: String,
    /// Whether the server is ready
    pub ready: bool,
    /// URL if connected to external server
    pub url: Option<String>,
    /// Model path if using sidecar
    pub model_path: Option<String>,
    /// Whether in embedding mode (sidecar only)
    pub is_embedding_mode: bool,
    /// Backend-owned target descriptor for the active runtime model.
    #[serde(default)]
    pub active_model_target: Option<String>,
    /// Backend-owned target descriptor for the dedicated embedding runtime model.
    #[serde(default)]
    pub embedding_model_target: Option<String>,
    /// Backend-owned lifecycle snapshot for the active runtime.
    #[serde(default)]
    pub active_runtime: Option<RuntimeLifecycleSnapshot>,
    /// Backend-owned lifecycle snapshot for the dedicated embedding runtime.
    #[serde(default)]
    pub embedding_runtime: Option<RuntimeLifecycleSnapshot>,
}

impl ServerModeInfo {
    pub fn runtime_fact_snapshots(&self) -> Vec<RuntimeFactSnapshot> {
        let mut facts = Vec::new();

        if let Some(active_runtime) = self.active_runtime.clone() {
            facts.push(RuntimeFactSnapshot::from_lifecycle(
                self.backend_key.clone(),
                self.active_model_target
                    .clone()
                    .or_else(|| self.model_path.clone()),
                None,
                active_runtime,
            ));
        } else if self.ready {
            facts.push(RuntimeFactSnapshot::absent_backend(
                self.backend_key.clone(),
                RuntimeFactAbsenceReason::Unsupported,
            ));
        } else {
            facts.push(RuntimeFactSnapshot::absent_backend(
                self.backend_key.clone(),
                RuntimeFactAbsenceReason::Unloaded,
            ));
        }

        if let Some(embedding_runtime) = self.embedding_runtime.clone() {
            facts.push(RuntimeFactSnapshot::from_lifecycle(
                self.backend_key.clone(),
                self.embedding_model_target.clone(),
                None,
                embedding_runtime,
            ));
        }

        facts
    }
}

/// Type identifier for masked prompts in JSON context values
pub const MASKED_PROMPT_TYPE: &str = "masked_prompt";

/// A segment of a masked prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSegment {
    /// The text content of this segment
    pub text: String,
    /// Whether this segment should be regenerated (true) or preserved as anchor (false)
    pub masked: bool,
}

/// A masked prompt consisting of segments, some masked for regeneration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskedPrompt {
    /// Type identifier, always "masked_prompt"
    #[serde(rename = "type")]
    pub prompt_type: String,
    /// The prompt segments
    pub segments: Vec<PromptSegment>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_segment_serde_roundtrip() {
        let segment = PromptSegment {
            text: "Hello world".to_string(),
            masked: true,
        };
        let json = serde_json::to_string(&segment).unwrap();
        let decoded: PromptSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.text, "Hello world");
        assert!(decoded.masked);
    }

    #[test]
    fn test_masked_prompt_serde_roundtrip() {
        let prompt = MaskedPrompt {
            prompt_type: MASKED_PROMPT_TYPE.to_string(),
            segments: vec![
                PromptSegment {
                    text: "The quick ".to_string(),
                    masked: false,
                },
                PromptSegment {
                    text: "brown fox".to_string(),
                    masked: true,
                },
                PromptSegment {
                    text: " jumps over".to_string(),
                    masked: false,
                },
            ],
        };
        let json = serde_json::to_string(&prompt).unwrap();
        let decoded: MaskedPrompt = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.prompt_type, "masked_prompt");
        assert_eq!(decoded.segments.len(), 3);
        assert!(!decoded.segments[0].masked);
        assert!(decoded.segments[1].masked);
        assert!(!decoded.segments[2].masked);
        assert_eq!(decoded.segments[0].text, "The quick ");
        assert_eq!(decoded.segments[1].text, "brown fox");
        assert_eq!(decoded.segments[2].text, " jumps over");
    }

    #[test]
    fn test_image_generation_request_serde_roundtrip_preserves_future_fields() {
        let request = ImageGenerationRequest {
            model: "Qwen/Qwen-Image".to_string(),
            prompt: "a red paper lantern in the rain".to_string(),
            negative_prompt: Some("blurry".to_string()),
            width: Some(1024),
            height: Some(1024),
            num_inference_steps: Some(30),
            guidance_scale: Some(4.0),
            seed: Some(42),
            scheduler: Some("flow_match_euler".to_string()),
            num_images_per_prompt: Some(1),
            init_image: None,
            mask_image: None,
            strength: None,
            extra_options: serde_json::json!({
                "true_cfg_scale": 4.0
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        let decoded: ImageGenerationRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.model, "Qwen/Qwen-Image");
        assert_eq!(decoded.seed, Some(42));
        assert_eq!(
            decoded.extra_options["true_cfg_scale"],
            serde_json::json!(4.0)
        );
    }

    #[test]
    fn test_image_generation_result_serde_roundtrip() {
        let result = ImageGenerationResult {
            images: vec![EncodedImage {
                data_base64: "aGVsbG8=".to_string(),
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(512),
            }],
            seed_used: Some(42),
            metadata: serde_json::json!({
                "scheduler": "flow_match_euler"
            }),
        };

        let json = serde_json::to_string(&result).unwrap();
        let decoded: ImageGenerationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.seed_used, Some(42));
        assert_eq!(decoded.images.len(), 1);
        assert_eq!(decoded.images[0].mime_type, "image/png");
        assert_eq!(
            decoded.metadata["scheduler"],
            serde_json::json!("flow_match_euler")
        );
    }

    #[test]
    fn typed_execution_request_serde_uses_task_and_input_contracts() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-typed-1".to_string()),
            task_id: InferenceTaskId::TextGeneration,
            model_ref: Some(PumasModelRef {
                model_id: "pumas://models/tiny".to_string(),
                revision: Some("rev-1".to_string()),
                selected_artifact_id: Some("main".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            }),
            model_name: None,
            runtime_hint: Some("pytorch".to_string()),
            input: InferenceExecutionInput::TextGeneration {
                prompt: Some("Hello".to_string()),
                system_prompt: Some("Be brief".to_string()),
                messages: Vec::new(),
                stream: true,
            },
            generation_options: Some(GenerationOptions {
                length: crate::model_contracts::LengthGenerationOptions {
                    max_new_tokens: Some(64),
                    ..Default::default()
                },
                ..Default::default()
            }),
            extra_options: Value::Null,
        };

        let encoded = serde_json::to_value(&request).unwrap();
        let decoded: InferenceExecutionRequest = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["task_id"], serde_json::json!("text_generation"));
        assert_eq!(
            encoded["input"]["input_type"],
            serde_json::json!("text_generation")
        );
        assert_eq!(
            encoded["generation_options"]["length"]["max_new_tokens"],
            serde_json::json!(64)
        );
        assert_eq!(decoded, request);
    }

    #[test]
    fn typed_execution_request_maps_openai_chat_at_edge_and_validates() {
        let request = ChatRequest {
            model: "tiny-chat".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: vec![ContentPart::Text {
                    text: "Hi".to_string(),
                }],
            }],
            stream: true,
            max_tokens: Some(32),
            temperature: Some(0.4),
        };

        let typed = InferenceExecutionRequest::from_openai_chat_request(
            Some("req-chat-1".to_string()),
            request,
        );

        typed.validate().expect("mapped chat request is valid");
        assert_eq!(typed.task_id, InferenceTaskId::ChatCompletion);
        assert_eq!(typed.model_name.as_deref(), Some("tiny-chat"));
        assert_eq!(
            typed
                .generation_options
                .as_ref()
                .and_then(|options| options.length.max_new_tokens),
            Some(32)
        );
        assert_eq!(
            typed
                .generation_options
                .as_ref()
                .and_then(|options| options.sampling.temperature),
            Some(0.4)
        );
    }

    #[test]
    fn typed_execution_request_validation_rejects_mismatched_task_and_input() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-invalid".to_string()),
            task_id: InferenceTaskId::Embedding,
            model_ref: None,
            model_name: Some("tiny".to_string()),
            runtime_hint: None,
            input: InferenceExecutionInput::TextGeneration {
                prompt: Some("Hi".to_string()),
                system_prompt: None,
                messages: Vec::new(),
                stream: false,
            },
            generation_options: None,
            extra_options: Value::Null,
        };

        match request.validate() {
            Err(InferenceExecutionRequestValidationError::TaskInputMismatch {
                task_id,
                input_type,
            }) => {
                assert_eq!(task_id, InferenceTaskId::Embedding);
                assert_eq!(input_type, "text_generation");
            }
            other => panic!("expected task/input mismatch, got {other:?}"),
        }
    }

    #[test]
    fn typed_execution_result_serde_keeps_diagnostics_and_usage() {
        let result = InferenceExecutionResult::TextGeneration {
            text: "Done".to_string(),
            usage: Some(InferenceUsage {
                prompt_tokens: Some(4),
                completion_tokens: Some(2),
                total_tokens: Some(6),
            }),
            cache_handle_id: Some("kv-1".to_string()),
            option_diagnostics: vec![OptionCompatibilityDiagnostic {
                option_path: "sampling.temperature".to_string(),
                state: crate::model_contracts::OptionSupportState::Honored,
                backend_key: Some("pytorch".to_string()),
                message: Some("mapped to Transformers temperature".to_string()),
            }],
        };

        let encoded = serde_json::to_value(&result).unwrap();
        let decoded: InferenceExecutionResult = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["result_type"], serde_json::json!("text_generation"));
        assert_eq!(encoded["usage"]["total_tokens"], serde_json::json!(6));
        assert_eq!(
            encoded["option_diagnostics"][0]["state"],
            serde_json::json!("honored")
        );
        assert_eq!(decoded, result);
    }

    #[test]
    fn inference_request_lifecycle_event_serde_uses_stable_contract() {
        let event = InferenceRequestLifecycleEvent {
            request_id: Some("req-1".to_string()),
            phase: InferenceLifecyclePhase::BackendExecution,
            kind: InferenceRequestLifecycleEventKind::CleanupCompleted,
            occurred_at_ms: 42,
            backend_key: Some("llama_cpp".to_string()),
            runtime_instance_id: Some("llama-main-1".to_string()),
            detail: Some("stream dropped by consumer".to_string()),
        };

        let encoded = serde_json::to_value(&event).unwrap();
        let decoded: InferenceRequestLifecycleEvent =
            serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["phase"], serde_json::json!("backend_execution"));
        assert_eq!(encoded["kind"], serde_json::json!("cleanup_completed"));
        assert_eq!(decoded, event);
    }

    #[test]
    fn runtime_lifecycle_snapshot_normalized_reason_preserves_explicit_reason() {
        let snapshot = RuntimeLifecycleSnapshot {
            lifecycle_decision_reason: Some("reused_embedding_runtime".to_string()),
            active: true,
            runtime_reused: Some(true),
            ..RuntimeLifecycleSnapshot::default()
        };

        assert_eq!(
            snapshot.normalized_lifecycle_decision_reason().as_deref(),
            Some("reused_embedding_runtime")
        );
    }

    #[test]
    fn runtime_lifecycle_snapshot_normalized_reason_infers_reuse() {
        let snapshot = RuntimeLifecycleSnapshot {
            active: true,
            runtime_reused: Some(true),
            ..RuntimeLifecycleSnapshot::default()
        };

        assert_eq!(
            snapshot.normalized_lifecycle_decision_reason().as_deref(),
            Some("runtime_reused")
        );
    }

    #[test]
    fn runtime_lifecycle_snapshot_normalized_reason_infers_ready() {
        let snapshot = RuntimeLifecycleSnapshot {
            active: true,
            runtime_reused: Some(false),
            ..RuntimeLifecycleSnapshot::default()
        };

        assert_eq!(
            snapshot.normalized_lifecycle_decision_reason().as_deref(),
            Some("runtime_ready")
        );
    }

    #[test]
    fn runtime_lifecycle_snapshot_normalized_reason_infers_start_failure() {
        let snapshot = RuntimeLifecycleSnapshot {
            active: false,
            last_error: Some("failed".to_string()),
            ..RuntimeLifecycleSnapshot::default()
        };

        assert_eq!(
            snapshot.normalized_lifecycle_decision_reason().as_deref(),
            Some("runtime_start_failed")
        );
    }

    #[test]
    fn runtime_fact_snapshot_normalizes_ready_reused_lifecycle() {
        let fact = RuntimeFactSnapshot::from_lifecycle(
            Some("llama_cpp".to_string()),
            Some("/models/qwen.gguf".to_string()),
            Some("cuda:0".to_string()),
            RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-1".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(true),
                lifecycle_decision_reason: None,
                active: true,
                last_error: None,
            },
        );

        assert_eq!(fact.backend_key.as_deref(), Some("llama_cpp"));
        assert_eq!(fact.runtime_id.as_deref(), Some("llama.cpp"));
        assert_eq!(fact.runtime_instance_id.as_deref(), Some("llama-main-1"));
        assert_eq!(
            fact.active_model_target.as_deref(),
            Some("/models/qwen.gguf")
        );
        assert_eq!(fact.resolved_device.as_deref(), Some("cuda:0"));
        assert_eq!(fact.readiness, RuntimeFactReadiness::Ready);
        assert_eq!(fact.reuse_result, RuntimeFactReuseResult::Reused);
        assert_eq!(
            fact.lifecycle_decision_reason.as_deref(),
            Some("runtime_reused")
        );
        assert_eq!(fact.absence_reason, None);
    }

    #[test]
    fn runtime_fact_snapshot_marks_active_warmup() {
        let fact = RuntimeFactSnapshot::from_lifecycle(
            Some("pytorch".to_string()),
            Some("Qwen/Qwen3-8B".to_string()),
            None,
            RuntimeLifecycleSnapshot {
                runtime_id: Some("PyTorch".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: None,
                runtime_reused: Some(false),
                active: true,
                ..RuntimeLifecycleSnapshot::default()
            },
        );

        assert_eq!(fact.readiness, RuntimeFactReadiness::Warming);
        assert_eq!(fact.reuse_result, RuntimeFactReuseResult::Started);
        assert_eq!(
            fact.lifecycle_decision_reason.as_deref(),
            Some("runtime_warming")
        );
    }

    #[test]
    fn runtime_fact_snapshot_marks_failed_absence() {
        let fact = RuntimeFactSnapshot::from_lifecycle(
            Some("candle".to_string()),
            Some("Qwen/Qwen3-8B".to_string()),
            None,
            RuntimeLifecycleSnapshot {
                runtime_id: Some("candle".to_string()),
                runtime_reused: None,
                active: false,
                last_error: Some("backend failed to start".to_string()),
                ..RuntimeLifecycleSnapshot::default()
            },
        );

        assert_eq!(fact.readiness, RuntimeFactReadiness::Failed);
        assert_eq!(fact.reuse_result, RuntimeFactReuseResult::Unknown);
        assert_eq!(fact.absence_reason, Some(RuntimeFactAbsenceReason::Failed));
        assert_eq!(
            fact.lifecycle_decision_reason.as_deref(),
            Some("runtime_start_failed")
        );
        assert_eq!(
            fact.last_backend_error.as_deref(),
            Some("backend failed to start")
        );
    }

    #[test]
    fn runtime_fact_absent_backend_defines_unloaded_and_unsupported_semantics() {
        let unloaded = RuntimeFactSnapshot::absent_backend(
            Some("llama_cpp".to_string()),
            RuntimeFactAbsenceReason::Unloaded,
        );
        let unsupported = RuntimeFactSnapshot::absent_backend(
            Some("candle".to_string()),
            RuntimeFactAbsenceReason::Unsupported,
        );

        assert_eq!(unloaded.readiness, RuntimeFactReadiness::Stopped);
        assert_eq!(
            unloaded.absence_reason,
            Some(RuntimeFactAbsenceReason::Unloaded)
        );
        assert_eq!(unsupported.readiness, RuntimeFactReadiness::Unsupported);
        assert_eq!(
            unsupported.reuse_result,
            RuntimeFactReuseResult::NotApplicable
        );
    }

    #[test]
    fn server_mode_info_projects_runtime_fact_snapshots() {
        let mode_info = ServerModeInfo {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            mode: "sidecar_inference".to_string(),
            ready: true,
            url: None,
            model_path: Some("/models/from-mode.gguf".to_string()),
            is_embedding_mode: false,
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-1".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: Some(RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("llama-embed-1".to_string()),
                runtime_reused: Some(true),
                active: true,
                ..RuntimeLifecycleSnapshot::default()
            }),
        };

        let facts = mode_info.runtime_fact_snapshots();

        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].backend_key.as_deref(), Some("llama_cpp"));
        assert_eq!(
            facts[0].active_model_target.as_deref(),
            Some("/models/qwen.gguf")
        );
        assert_eq!(facts[0].readiness, RuntimeFactReadiness::Ready);
        assert_eq!(
            facts[1].active_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert_eq!(facts[1].reuse_result, RuntimeFactReuseResult::Reused);
    }

    #[test]
    fn server_mode_info_projects_absent_runtime_when_backend_does_not_report_lifecycle() {
        let mode_info = ServerModeInfo {
            backend_name: Some("Candle".to_string()),
            backend_key: Some("candle".to_string()),
            mode: "sidecar_inference".to_string(),
            ready: true,
            url: None,
            model_path: None,
            is_embedding_mode: false,
            active_model_target: None,
            embedding_model_target: None,
            active_runtime: None,
            embedding_runtime: None,
        };

        let facts = mode_info.runtime_fact_snapshots();

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].backend_key.as_deref(), Some("candle"));
        assert_eq!(facts[0].readiness, RuntimeFactReadiness::Unsupported);
        assert_eq!(
            facts[0].absence_reason,
            Some(RuntimeFactAbsenceReason::Unsupported)
        );
    }

    #[test]
    fn runtime_fact_snapshot_serde_uses_stable_snake_case_contract() {
        let fact = RuntimeFactSnapshot::absent_backend(
            Some("mlx".to_string()),
            RuntimeFactAbsenceReason::Unsupported,
        );

        let encoded = serde_json::to_value(&fact).unwrap();

        assert_eq!(encoded["backend_key"], serde_json::json!("mlx"));
        assert_eq!(encoded["readiness"], serde_json::json!("unsupported"));
        assert_eq!(encoded["reuse_result"], serde_json::json!("not_applicable"));
        assert_eq!(encoded["absence_reason"], serde_json::json!("unsupported"));
    }
}
