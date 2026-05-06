//! Common types for inference operations

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model_contracts::{
    resolve_task_registry_entry, resolve_task_registry_entry_from_evidence, GenerationOptions,
    InferenceExecutionInputKind, InferenceExecutionResultKind, InferenceLifecyclePhase,
    InferenceTaskId, OptionCompatibilityDiagnostic, PumasModelRef, ResolvedModelPackageFacts,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model_package_facts: Option<ResolvedModelPackageFacts>,
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
        let generation_options = if request.max_tokens.is_some()
            || request.temperature.is_some()
            || request.top_p.is_some()
            || request.top_k.is_some()
        {
            Some(GenerationOptions {
                length: crate::model_contracts::LengthGenerationOptions {
                    max_new_tokens: request.max_tokens,
                    ..Default::default()
                },
                sampling: crate::model_contracts::SamplingGenerationOptions {
                    temperature: request.temperature,
                    top_p: request.top_p,
                    top_k: request.top_k,
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
            resolved_model_package_facts: None,
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
        if let InferenceExecutionInput::AudioTranscription { request } = &self.input {
            request
                .validate_audio_source()
                .map_err(|()| InferenceExecutionRequestValidationError::MissingAudioInput)?;
        }

        let contract = resolve_task_registry_entry(self.task_id.canonical_label())
            .and_then(|entry| entry.request_contract())
            .ok_or_else(
                || InferenceExecutionRequestValidationError::UnsupportedTask {
                    task_id: self.task_id.clone(),
                },
            )?;
        if !contract.execution_supported {
            return Err(InferenceExecutionRequestValidationError::UnsupportedTask {
                task_id: self.task_id.clone(),
            });
        }
        if contract.input_kind != self.input.input_kind() {
            return Err(
                InferenceExecutionRequestValidationError::TaskInputMismatch {
                    task_id: self.task_id.clone(),
                    input_type: self.input.input_type_label(),
                },
            );
        }
        if let Some(package_facts) = &self.resolved_model_package_facts {
            if let Ok(package_task) = resolve_task_registry_entry_from_evidence(&package_facts.task)
            {
                if package_task.task_id != self.task_id {
                    return Err(
                        InferenceExecutionRequestValidationError::PackageTaskMismatch {
                            request_task_id: self.task_id.clone(),
                            package_task_id: package_task.task_id,
                            model_id: package_facts.model_ref.model_id.clone(),
                        },
                    );
                }
            }
        }

        match &self.input {
            InferenceExecutionInput::TextGeneration {
                prompt, messages, ..
            } => {
                if prompt
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                    && messages.is_empty()
                {
                    return Err(InferenceExecutionRequestValidationError::MissingTextInput);
                }
                Ok(())
            }
            InferenceExecutionInput::Embedding { texts } => {
                if texts.is_empty() {
                    return Err(InferenceExecutionRequestValidationError::EmptyEmbeddingTexts);
                }
                if let Some((index, _)) = texts
                    .iter()
                    .enumerate()
                    .find(|(_, text)| text.trim().is_empty())
                {
                    return Err(
                        InferenceExecutionRequestValidationError::BlankEmbeddingText { index },
                    );
                }
                Ok(())
            }
            InferenceExecutionInput::Rerank {
                query, documents, ..
            } => {
                if query.trim().is_empty() {
                    return Err(InferenceExecutionRequestValidationError::EmptyRerankQuery);
                }
                if documents.is_empty() {
                    return Err(InferenceExecutionRequestValidationError::EmptyRerankDocuments);
                }
                if let Some((index, _)) = documents
                    .iter()
                    .enumerate()
                    .find(|(_, document)| document.trim().is_empty())
                {
                    return Err(
                        InferenceExecutionRequestValidationError::BlankRerankDocument { index },
                    );
                }
                Ok(())
            }
            InferenceExecutionInput::ImageGeneration { .. } => Ok(()),
            InferenceExecutionInput::AudioTranscription { .. } => Ok(()),
            InferenceExecutionInput::ImageUnderstanding { .. }
            | InferenceExecutionInput::DepthEstimation { .. }
            | InferenceExecutionInput::VideoUnderstanding { .. }
            | InferenceExecutionInput::MultimodalGeneration { .. } => Ok(()),
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
    #[error("embedding text at index {index} must not be blank")]
    BlankEmbeddingText { index: usize },
    #[error("rerank execution requires a query")]
    EmptyRerankQuery,
    #[error("rerank execution requires at least one document")]
    EmptyRerankDocuments,
    #[error("rerank document at index {index} must not be blank")]
    BlankRerankDocument { index: usize },
    #[error("audio transcription requires encoded audio or an audio artifact reference")]
    MissingAudioInput,
    #[error("task {task_id:?} does not match input type {input_type}")]
    TaskInputMismatch {
        task_id: InferenceTaskId,
        input_type: &'static str,
    },
    #[error("task {task_id:?} is not supported by the typed execution request contract")]
    UnsupportedTask { task_id: InferenceTaskId },
    #[error(
        "request task {request_task_id:?} does not match resolved package task {package_task_id:?} for model {model_id}"
    )]
    PackageTaskMismatch {
        request_task_id: InferenceTaskId,
        package_task_id: InferenceTaskId,
        model_id: String,
    },
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
    AudioTranscription {
        request: AudioTranscriptionRequest,
    },
    ImageUnderstanding {
        request: ImageUnderstandingRequest,
    },
    DepthEstimation {
        request: DepthEstimationRequest,
    },
    VideoUnderstanding {
        request: VideoUnderstandingRequest,
    },
    MultimodalGeneration {
        request: MultimodalGenerationRequest,
    },
}

impl InferenceExecutionInput {
    #[must_use]
    pub fn input_type_label(&self) -> &'static str {
        self.input_kind().canonical_label()
    }

    #[must_use]
    pub fn input_kind(&self) -> InferenceExecutionInputKind {
        match self {
            Self::TextGeneration { .. } => InferenceExecutionInputKind::TextGeneration,
            Self::Embedding { .. } => InferenceExecutionInputKind::Embedding,
            Self::Rerank { .. } => InferenceExecutionInputKind::Rerank,
            Self::ImageGeneration { .. } => InferenceExecutionInputKind::ImageGeneration,
            Self::AudioTranscription { .. } => InferenceExecutionInputKind::AudioTranscription,
            Self::ImageUnderstanding { .. } => InferenceExecutionInputKind::ImageUnderstanding,
            Self::DepthEstimation { .. } => InferenceExecutionInputKind::DepthEstimation,
            Self::VideoUnderstanding { .. } => InferenceExecutionInputKind::VideoUnderstanding,
            Self::MultimodalGeneration { .. } => InferenceExecutionInputKind::MultimodalGeneration,
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
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
    Rerank {
        response: RerankResponse,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
    ImageGeneration {
        result: ImageGenerationResult,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
    AudioTranscription {
        result: AudioTranscriptionResult,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
    ImageUnderstanding {
        result: TextUnderstandingResult,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
    DepthEstimation {
        result: DepthEstimationResult,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
    VideoUnderstanding {
        result: TextUnderstandingResult,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
    MultimodalGeneration {
        result: TextUnderstandingResult,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    },
}

impl InferenceExecutionResult {
    #[must_use]
    pub fn result_kind(&self) -> InferenceExecutionResultKind {
        match self {
            Self::TextGeneration { .. } => InferenceExecutionResultKind::TextGeneration,
            Self::Embedding { .. } => InferenceExecutionResultKind::Embedding,
            Self::Rerank { .. } => InferenceExecutionResultKind::Rerank,
            Self::ImageGeneration { .. } => InferenceExecutionResultKind::ImageGeneration,
            Self::AudioTranscription { .. } => InferenceExecutionResultKind::AudioTranscription,
            Self::ImageUnderstanding { .. } => InferenceExecutionResultKind::ImageUnderstanding,
            Self::DepthEstimation { .. } => InferenceExecutionResultKind::DepthEstimation,
            Self::VideoUnderstanding { .. } => InferenceExecutionResultKind::VideoUnderstanding,
            Self::MultimodalGeneration { .. } => InferenceExecutionResultKind::MultimodalGeneration,
        }
    }

    #[must_use]
    pub fn option_diagnostics(&self) -> &[OptionCompatibilityDiagnostic] {
        match self {
            Self::TextGeneration {
                option_diagnostics, ..
            }
            | Self::Embedding {
                option_diagnostics, ..
            }
            | Self::Rerank {
                option_diagnostics, ..
            }
            | Self::ImageGeneration {
                option_diagnostics, ..
            }
            | Self::AudioTranscription {
                option_diagnostics, ..
            }
            | Self::ImageUnderstanding {
                option_diagnostics, ..
            }
            | Self::DepthEstimation {
                option_diagnostics, ..
            }
            | Self::VideoUnderstanding {
                option_diagnostics, ..
            }
            | Self::MultimodalGeneration {
                option_diagnostics, ..
            } => option_diagnostics,
        }
    }

    #[must_use]
    pub fn usage(&self) -> Option<&InferenceUsage> {
        match self {
            Self::TextGeneration { usage, .. } | Self::Embedding { usage, .. } => usage.as_ref(),
            Self::Rerank { .. }
            | Self::ImageGeneration { .. }
            | Self::AudioTranscription { .. }
            | Self::ImageUnderstanding { .. }
            | Self::DepthEstimation { .. }
            | Self::VideoUnderstanding { .. }
            | Self::MultimodalGeneration { .. } => None,
        }
    }

    #[must_use]
    pub fn cache_handle_id(&self) -> Option<&str> {
        match self {
            Self::TextGeneration {
                cache_handle_id, ..
            } => cache_handle_id.as_deref(),
            Self::Embedding { .. }
            | Self::Rerank { .. }
            | Self::ImageGeneration { .. }
            | Self::AudioTranscription { .. }
            | Self::ImageUnderstanding { .. }
            | Self::DepthEstimation { .. }
            | Self::VideoUnderstanding { .. }
            | Self::MultimodalGeneration { .. } => None,
        }
    }
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

/// Base64-encoded audio payload used by audio transcription requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedAudio {
    /// Base64-encoded audio bytes.
    pub data_base64: String,
    /// MIME type describing the encoded audio payload.
    pub mime_type: String,
    /// Optional sample rate in hertz when known by the caller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate_hz: Option<u32>,
}

/// Base64-encoded video payload reserved for video understanding contracts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EncodedVideo {
    /// Base64-encoded video bytes.
    pub data_base64: String,
    /// MIME type describing the encoded video payload.
    pub mime_type: String,
    /// Optional duration in seconds when known by the caller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f32>,
    /// Optional frame count when known by the caller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_count: Option<u32>,
}

/// Image-to-text request contract reserved for future vision-language backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageUnderstandingRequest {
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<EncodedImage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

/// Image depth-estimation request contract reserved for future depth backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepthEstimationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<EncodedImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

/// Video-to-text request contract reserved for future video-language backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoUnderstandingRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<EncodedVideo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

/// One typed multimodal input part for future generation/perception contracts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "part_type", rename_all = "snake_case")]
pub enum MultimodalInputPart {
    Text {
        text: String,
    },
    Image {
        image: EncodedImage,
    },
    Audio {
        audio: EncodedAudio,
    },
    Video {
        video: EncodedVideo,
    },
    Artifact {
        modality: crate::model_contracts::InferenceModality,
        artifact_ref: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

/// Multimodal generation request contract reserved for future backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultimodalGenerationRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<MultimodalInputPart>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

/// Text result returned by understanding-style contract-only tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextUnderstandingResult {
    pub text: String,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

/// Depth-estimation result contract reserved for future depth backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepthEstimationResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_map: Option<EncodedImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_map_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point_cloud_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

/// Speech-to-text request contract used by ASR-capable backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioTranscriptionRequest {
    /// Backend-specific model identifier or path.
    pub model: String,
    /// In-memory audio payload. Large media should normally flow through
    /// artifact references instead of durable diagnostics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<EncodedAudio>,
    /// Optional artifact reference for host-owned audio payloads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_ref: Option<String>,
    /// Optional language hint such as `en`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Optional prompt/context hint for transcription.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Optional backend task hint such as `transcribe` or `translate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// Optional chunk size in seconds for long-form transcription.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_length_s: Option<f32>,
    /// Backend/model-specific append-only options.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

impl AudioTranscriptionRequest {
    fn validate_audio_source(&self) -> Result<(), ()> {
        if self
            .audio
            .as_ref()
            .is_some_and(|audio| !audio.data_base64.trim().is_empty())
            || self
                .audio_ref
                .as_deref()
                .is_some_and(|audio_ref| !audio_ref.trim().is_empty())
        {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// Speech-to-text response contract returned by ASR-capable backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioTranscriptionResult {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<AudioTranscriptionSegment>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

/// Bounded timing segment returned by audio transcription when available.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioTranscriptionSegment {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_seconds: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_seconds: Option<f32>,
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

/// Bounded backend/model compatibility status summary for diagnostics.
///
/// This is ledger-neutral metadata. It describes factual compatibility checks
/// that already happened at the inference boundary and must not be used as
/// scheduler policy by itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct InferenceCompatibilityReportSummary {
    pub status: String,
    pub compatible: bool,
    pub task: String,
    pub model_source: String,
    pub preprocessing: String,
    pub postprocessing: String,
}

/// Bounded backend/model compatibility issue summary for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct InferenceCompatibilityIssueSummary {
    pub kind: String,
    pub phase: InferenceLifecyclePhase,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
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
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_network_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_artifact_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<InferenceUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_handle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_error_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_report: Option<InferenceCompatibilityReportSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
}

/// Return a bounded artifact reference suitable for lifecycle diagnostics.
///
/// Stable logical refs such as `artifact://...` are preserved. Local path-like
/// values are dropped so producers and ledger adapters do not persist absolute
/// paths, relative paths, home paths, file URLs, or Windows drive paths.
#[must_use]
pub fn bounded_inference_artifact_ref(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || looks_like_local_artifact_ref(value) {
        None
    } else {
        Some(value.to_string())
    }
}

#[must_use]
pub fn looks_like_local_artifact_ref(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("./")
        || value.starts_with("../")
        || value.starts_with("~/")
        || value
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"))
        || value.as_bytes().get(1) == Some(&b':')
            && value
                .as_bytes()
                .get(2)
                .is_some_and(|byte| *byte == b'/' || *byte == b'\\')
            && value
                .as_bytes()
                .first()
                .is_some_and(|byte| byte.is_ascii_alphabetic())
}

/// Error returned by a host-owned lifecycle sink when it cannot persist or
/// forward a lifecycle fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceRequestLifecycleEventSinkError {
    pub code: String,
    pub message: String,
}

impl InferenceRequestLifecycleEventSinkError {
    pub fn diagnostics_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "diagnostics_unavailable".to_string(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for InferenceRequestLifecycleEventSinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for InferenceRequestLifecycleEventSinkError {}

/// Synchronous sink for request lifecycle facts.
pub trait InferenceRequestLifecycleEventSink: Send + Sync {
    fn record(
        &self,
        event: InferenceRequestLifecycleEvent,
    ) -> Result<(), InferenceRequestLifecycleEventSinkError>;
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
    /// Backend-owned explicit non-auto device fact for the active runtime.
    #[serde(default)]
    pub active_resolved_device: Option<String>,
    /// Backend-owned target descriptor for the dedicated embedding runtime model.
    #[serde(default)]
    pub embedding_model_target: Option<String>,
    /// Backend-owned explicit non-auto device fact for the dedicated embedding runtime.
    #[serde(default)]
    pub embedding_resolved_device: Option<String>,
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
                self.active_resolved_device.clone(),
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
                self.embedding_resolved_device.clone(),
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
    use std::collections::BTreeSet;

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
    fn test_audio_transcription_request_result_serde_roundtrip() {
        let request = AudioTranscriptionRequest {
            model: "openai/whisper-tiny".to_string(),
            audio: Some(EncodedAudio {
                data_base64: "UklGRg==".to_string(),
                mime_type: "audio/wav".to_string(),
                sample_rate_hz: Some(16_000),
            }),
            audio_ref: None,
            language: Some("en".to_string()),
            prompt: Some("technical vocabulary".to_string()),
            task: Some("transcribe".to_string()),
            chunk_length_s: Some(30.0),
            extra_options: serde_json::json!({
                "return_timestamps": true
            }),
        };
        let result = AudioTranscriptionResult {
            text: "hello world".to_string(),
            language: Some("en".to_string()),
            duration_seconds: Some(1.25),
            segments: vec![AudioTranscriptionSegment {
                text: "hello".to_string(),
                start_seconds: Some(0.0),
                end_seconds: Some(0.5),
            }],
            metadata: serde_json::json!({
                "backend": "pytorch"
            }),
        };

        let request_json = serde_json::to_string(&request).unwrap();
        let result_json = serde_json::to_string(&result).unwrap();
        let decoded_request: AudioTranscriptionRequest =
            serde_json::from_str(&request_json).unwrap();
        let decoded_result: AudioTranscriptionResult = serde_json::from_str(&result_json).unwrap();

        assert_eq!(decoded_request.model, "openai/whisper-tiny");
        assert_eq!(
            decoded_request
                .audio
                .as_ref()
                .and_then(|audio| audio.sample_rate_hz),
            Some(16_000)
        );
        assert_eq!(decoded_result.text, "hello world");
        assert_eq!(decoded_result.segments.len(), 1);
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
            resolved_model_package_facts: None,
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
    fn typed_execution_embedding_request_serde_uses_stable_contract() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-embedding-1".to_string()),
            task_id: InferenceTaskId::Embedding,
            model_ref: None,
            model_name: Some("sentence-transformers/tiny".to_string()),
            runtime_hint: Some("candle".to_string()),
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::Embedding {
                texts: vec!["first".to_string(), "second".to_string()],
            },
            generation_options: None,
            extra_options: serde_json::json!({
                "normalize": true
            }),
        };

        let encoded = serde_json::to_value(&request).unwrap();
        let decoded: InferenceExecutionRequest = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["task_id"], serde_json::json!("embedding"));
        assert_eq!(
            encoded["input"]["input_type"],
            serde_json::json!("embedding")
        );
        assert_eq!(encoded["input"]["texts"][1], serde_json::json!("second"));
        assert_eq!(
            encoded["extra_options"]["normalize"],
            serde_json::json!(true)
        );
        assert_eq!(decoded, request);
    }

    #[test]
    fn typed_execution_rerank_request_serde_uses_stable_contract() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-rerank-1".to_string()),
            task_id: InferenceTaskId::Rerank,
            model_ref: None,
            model_name: Some("reranker/tiny".to_string()),
            runtime_hint: Some("pytorch".to_string()),
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::Rerank {
                query: "needle".to_string(),
                documents: vec!["hay".to_string(), "needle document".to_string()],
                top_n: Some(1),
                return_documents: true,
            },
            generation_options: None,
            extra_options: Value::Null,
        };

        let encoded = serde_json::to_value(&request).unwrap();
        let decoded: InferenceExecutionRequest = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["task_id"], serde_json::json!("rerank"));
        assert_eq!(encoded["input"]["input_type"], serde_json::json!("rerank"));
        assert_eq!(encoded["input"]["top_n"], serde_json::json!(1));
        assert_eq!(
            encoded["input"]["return_documents"],
            serde_json::json!(true)
        );
        assert_eq!(decoded, request);
    }

    #[test]
    fn typed_execution_image_generation_request_serde_uses_stable_contract() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-image-1".to_string()),
            task_id: InferenceTaskId::ImageGeneration,
            model_ref: None,
            model_name: Some("diffusers/tiny".to_string()),
            runtime_hint: Some("diffusers".to_string()),
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::ImageGeneration {
                request: ImageGenerationRequest {
                    model: "diffusers/tiny".to_string(),
                    prompt: "calm lake".to_string(),
                    negative_prompt: Some("low quality".to_string()),
                    width: Some(512),
                    height: Some(512),
                    num_inference_steps: Some(20),
                    guidance_scale: Some(7.5),
                    seed: Some(42),
                    scheduler: Some("euler".to_string()),
                    num_images_per_prompt: Some(2),
                    init_image: None,
                    mask_image: None,
                    strength: None,
                    extra_options: Value::Null,
                },
            },
            generation_options: None,
            extra_options: Value::Null,
        };

        let encoded = serde_json::to_value(&request).unwrap();
        let decoded: InferenceExecutionRequest = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["task_id"], serde_json::json!("image_generation"));
        assert_eq!(
            encoded["input"]["input_type"],
            serde_json::json!("image_generation")
        );
        assert_eq!(
            encoded["input"]["request"]["num_inference_steps"],
            serde_json::json!(20)
        );
        assert_eq!(
            encoded["input"]["request"]["scheduler"],
            serde_json::json!("euler")
        );
        assert_eq!(decoded, request);
    }

    #[test]
    fn typed_execution_request_wire_shape_separates_stable_and_backend_local_fields() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-boundary-1".to_string()),
            task_id: InferenceTaskId::TextGeneration,
            model_ref: None,
            model_name: Some("tiny-model".to_string()),
            runtime_hint: Some("pytorch".to_string()),
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::TextGeneration {
                prompt: Some("Hello".to_string()),
                system_prompt: None,
                messages: Vec::new(),
                stream: false,
            },
            generation_options: Some(GenerationOptions {
                backend_extensions: [(
                    "transformers:renormalize_logits".to_string(),
                    serde_json::json!(true),
                )]
                .into_iter()
                .collect(),
                ..Default::default()
            }),
            extra_options: serde_json::json!({
                "adapter:opaque_option": true
            }),
        };

        let encoded = serde_json::to_value(&request).unwrap();
        let top_level_keys: BTreeSet<&str> = encoded
            .as_object()
            .expect("request should encode as object")
            .keys()
            .map(String::as_str)
            .collect();

        assert_eq!(
            top_level_keys,
            BTreeSet::from([
                "extra_options",
                "generation_options",
                "input",
                "model_name",
                "request_id",
                "runtime_hint",
                "task_id",
            ])
        );
        assert_eq!(
            encoded["generation_options"]["backend_extensions"]["transformers:renormalize_logits"],
            serde_json::json!(true)
        );
        assert_eq!(
            encoded["extra_options"]["adapter:opaque_option"],
            serde_json::json!(true)
        );
        assert!(encoded.get("transformers_kwargs").is_none());
        assert!(encoded.get("backend_cli_flags").is_none());
        assert!(encoded.get("scheduler_policy").is_none());
    }

    #[test]
    fn typed_execution_request_wire_shape_defaults_and_ignores_unknown_fields() {
        let encoded = serde_json::json!({
            "task_id": "embedding",
            "model_name": "sentence-transformers/tiny",
            "input": {
                "input_type": "embedding",
                "texts": ["first"]
            },
            "future_public_field": {
                "ignored": true
            }
        });

        let decoded: InferenceExecutionRequest = serde_json::from_value(encoded).unwrap();

        assert_eq!(decoded.request_id, None);
        assert_eq!(decoded.task_id, InferenceTaskId::Embedding);
        assert_eq!(decoded.model_ref, None);
        assert_eq!(decoded.runtime_hint, None);
        assert_eq!(decoded.resolved_model_package_facts, None);
        assert_eq!(decoded.generation_options, None);
        assert_eq!(decoded.extra_options, Value::Null);
        assert_eq!(
            decoded.input,
            InferenceExecutionInput::Embedding {
                texts: vec!["first".to_string()]
            }
        );
        decoded
            .validate()
            .expect("decoded minimal typed request should validate");
    }

    #[test]
    fn typed_execution_audio_transcription_serde_uses_stable_contract() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-audio-1".to_string()),
            task_id: InferenceTaskId::AudioTranscription,
            model_ref: None,
            model_name: Some("openai/whisper-tiny".to_string()),
            runtime_hint: Some("pytorch".to_string()),
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::AudioTranscription {
                request: AudioTranscriptionRequest {
                    model: "openai/whisper-tiny".to_string(),
                    audio: None,
                    audio_ref: Some("artifact://audio.wav".to_string()),
                    language: Some("en".to_string()),
                    prompt: None,
                    task: Some("transcribe".to_string()),
                    chunk_length_s: None,
                    extra_options: Value::Null,
                },
            },
            generation_options: None,
            extra_options: Value::Null,
        };
        let result = InferenceExecutionResult::AudioTranscription {
            result: AudioTranscriptionResult {
                text: "transcribed text".to_string(),
                language: Some("en".to_string()),
                duration_seconds: None,
                segments: Vec::new(),
                metadata: Value::Null,
            },
            option_diagnostics: Vec::new(),
        };

        let encoded_request = serde_json::to_value(&request).unwrap();
        let decoded_request: InferenceExecutionRequest =
            serde_json::from_value(encoded_request.clone()).unwrap();
        let encoded_result = serde_json::to_value(&result).unwrap();
        let decoded_result: InferenceExecutionResult =
            serde_json::from_value(encoded_result.clone()).unwrap();

        assert_eq!(
            encoded_request["input"]["input_type"],
            serde_json::json!("audio_transcription")
        );
        assert_eq!(
            encoded_request["input"]["request"]["audio_ref"],
            serde_json::json!("artifact://audio.wav")
        );
        assert_eq!(
            encoded_result["result_type"],
            serde_json::json!("audio_transcription")
        );
        assert_eq!(
            decoded_result.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::AudioTranscription
        );
        assert_eq!(decoded_request, request);
        assert_eq!(decoded_result, result);
    }

    #[test]
    fn typed_execution_understanding_roadmap_contracts_use_stable_wire_shapes() {
        let image_request = InferenceExecutionRequest {
            request_id: Some("req-image-understanding".to_string()),
            task_id: InferenceTaskId::ImageUnderstanding,
            model_ref: None,
            model_name: Some("vision-language-roadmap".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::ImageUnderstanding {
                request: ImageUnderstandingRequest {
                    prompt: "describe this image".to_string(),
                    images: Vec::new(),
                    image_refs: vec!["artifact://image-a.png".to_string()],
                    extra_options: Value::Null,
                },
            },
            generation_options: None,
            extra_options: Value::Null,
        };
        let video_request = InferenceExecutionRequest {
            request_id: Some("req-video-understanding".to_string()),
            task_id: InferenceTaskId::VideoUnderstanding,
            model_ref: None,
            model_name: Some("video-language-roadmap".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::VideoUnderstanding {
                request: VideoUnderstandingRequest {
                    prompt: "summarize this clip".to_string(),
                    video: None,
                    video_ref: Some("artifact://clip.mp4".to_string()),
                    extra_options: serde_json::json!({
                        "max_frames": 8
                    }),
                },
            },
            generation_options: None,
            extra_options: Value::Null,
        };
        let depth_request = InferenceExecutionRequest {
            request_id: Some("req-depth-estimation".to_string()),
            task_id: InferenceTaskId::DepthEstimation,
            model_ref: None,
            model_name: Some("depth-roadmap".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::DepthEstimation {
                request: DepthEstimationRequest {
                    image: None,
                    image_ref: Some("artifact://image-a.png".to_string()),
                    extra_options: serde_json::json!({
                        "output_format": "depth_map"
                    }),
                },
            },
            generation_options: None,
            extra_options: Value::Null,
        };
        let multimodal_request = InferenceExecutionRequest {
            request_id: Some("req-multimodal".to_string()),
            task_id: InferenceTaskId::MultimodalGeneration,
            model_ref: None,
            model_name: Some("multimodal-roadmap".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::MultimodalGeneration {
                request: MultimodalGenerationRequest {
                    parts: vec![
                        MultimodalInputPart::Text {
                            text: "compare".to_string(),
                        },
                        MultimodalInputPart::Artifact {
                            modality: crate::model_contracts::InferenceModality::Image,
                            artifact_ref: "artifact://image-a.png".to_string(),
                            mime_type: Some("image/png".to_string()),
                        },
                    ],
                    extra_options: Value::Null,
                },
            },
            generation_options: None,
            extra_options: Value::Null,
        };
        let result = InferenceExecutionResult::VideoUnderstanding {
            result: TextUnderstandingResult {
                text: "a short clip".to_string(),
                metadata: serde_json::json!({
                    "frames_sampled": 8
                }),
            },
            option_diagnostics: Vec::new(),
        };
        let depth_result = InferenceExecutionResult::DepthEstimation {
            result: DepthEstimationResult {
                depth_map: None,
                depth_map_ref: Some("artifact://depth-map.png".to_string()),
                point_cloud_ref: Some("artifact://depth.ply".to_string()),
                metadata: serde_json::json!({
                    "depth_units": "relative"
                }),
            },
            option_diagnostics: Vec::new(),
        };

        for request in [
            image_request,
            video_request,
            depth_request,
            multimodal_request,
        ] {
            let encoded = serde_json::to_value(&request).unwrap();
            let decoded: InferenceExecutionRequest = serde_json::from_value(encoded).unwrap();

            match decoded.validate() {
                Err(InferenceExecutionRequestValidationError::UnsupportedTask { task_id }) => {
                    assert_eq!(task_id, decoded.task_id)
                }
                other => panic!("expected unsupported roadmap task, got {other:?}"),
            }
            assert_eq!(decoded, request);
        }

        let encoded_result = serde_json::to_value(&result).unwrap();
        let decoded_result: InferenceExecutionResult =
            serde_json::from_value(encoded_result.clone()).unwrap();

        assert_eq!(
            encoded_result["result_type"],
            serde_json::json!("video_understanding")
        );
        assert_eq!(
            decoded_result.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::VideoUnderstanding
        );
        assert_eq!(decoded_result, result);

        let encoded_depth_result = serde_json::to_value(&depth_result).unwrap();
        let decoded_depth_result: InferenceExecutionResult =
            serde_json::from_value(encoded_depth_result.clone()).unwrap();
        assert_eq!(
            encoded_depth_result["result_type"],
            serde_json::json!("depth_estimation")
        );
        assert_eq!(
            decoded_depth_result.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::DepthEstimation
        );
        assert_eq!(decoded_depth_result, depth_result);
    }

    #[test]
    fn typed_depth_estimation_wire_shape_defaults_and_ignores_unknown_fields() {
        let raw_request = serde_json::json!({
            "request_id": "req-depth-estimation",
            "task_id": "depth_estimation",
            "model_name": "depth-roadmap",
            "input": {
                "input_type": "depth_estimation",
                "request": {
                    "image_ref": "artifact://image-a.png",
                    "future_transformers_depth_field": {
                        "ignored": true
                    }
                },
                "future_input_field": true
            },
            "future_request_field": true
        });

        let request: InferenceExecutionRequest =
            serde_json::from_value(raw_request).expect("depth estimation request decodes");
        assert_eq!(request.task_id, InferenceTaskId::DepthEstimation);
        assert_eq!(
            request.input.input_kind(),
            crate::model_contracts::InferenceExecutionInputKind::DepthEstimation
        );
        let InferenceExecutionInput::DepthEstimation { request: depth } = &request.input else {
            panic!("expected depth estimation input");
        };
        assert!(depth.image.is_none());
        assert_eq!(depth.image_ref.as_deref(), Some("artifact://image-a.png"));
        assert_eq!(depth.extra_options, Value::Null);
        match request.validate() {
            Err(InferenceExecutionRequestValidationError::UnsupportedTask { task_id }) => {
                assert_eq!(task_id, InferenceTaskId::DepthEstimation);
            }
            other => panic!("expected unsupported roadmap task, got {other:?}"),
        }

        let encoded_request = serde_json::to_value(&request).expect("depth request encodes");
        assert_eq!(
            encoded_request["input"]["input_type"],
            serde_json::json!("depth_estimation")
        );
        assert_eq!(
            encoded_request["input"]["request"]["image_ref"],
            serde_json::json!("artifact://image-a.png")
        );
        assert!(encoded_request["input"]["request"].get("image").is_none());
        assert!(encoded_request["input"]["request"]
            .get("extra_options")
            .is_none());
        assert!(encoded_request["input"]["request"]
            .get("future_transformers_depth_field")
            .is_none());
        assert!(encoded_request["input"].get("future_input_field").is_none());
        assert!(encoded_request.get("future_request_field").is_none());

        let raw_result = serde_json::json!({
            "result_type": "depth_estimation",
            "result": {
                "depth_map_ref": "artifact://depth-map.png",
                "future_result_field": true
            },
            "future_envelope_field": true
        });
        let result: InferenceExecutionResult =
            serde_json::from_value(raw_result).expect("depth estimation result decodes");
        assert_eq!(
            result.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::DepthEstimation
        );
        let InferenceExecutionResult::DepthEstimation {
            result: depth_result,
            option_diagnostics,
        } = &result
        else {
            panic!("expected depth estimation result");
        };
        assert_eq!(
            depth_result.depth_map_ref.as_deref(),
            Some("artifact://depth-map.png")
        );
        assert!(depth_result.depth_map.is_none());
        assert!(depth_result.point_cloud_ref.is_none());
        assert_eq!(depth_result.metadata, Value::Null);
        assert!(option_diagnostics.is_empty());

        let encoded_result = serde_json::to_value(&result).expect("depth result encodes");
        assert_eq!(
            encoded_result["result_type"],
            serde_json::json!("depth_estimation")
        );
        assert_eq!(
            encoded_result["result"]["depth_map_ref"],
            serde_json::json!("artifact://depth-map.png")
        );
        assert!(encoded_result["result"].get("depth_map").is_none());
        assert!(encoded_result["result"].get("point_cloud_ref").is_none());
        assert!(encoded_result["result"].get("metadata").is_none());
        assert!(encoded_result["result"]
            .get("future_result_field")
            .is_none());
        assert!(encoded_result.get("option_diagnostics").is_none());
        assert!(encoded_result.get("future_envelope_field").is_none());
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
            top_p: Some(0.9),
            top_k: Some(40),
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
        assert_eq!(
            typed
                .generation_options
                .as_ref()
                .and_then(|options| options.sampling.top_p),
            Some(0.9)
        );
        assert_eq!(
            typed
                .generation_options
                .as_ref()
                .and_then(|options| options.sampling.top_k),
            Some(40)
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
            resolved_model_package_facts: None,
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
    fn typed_execution_request_validation_rejects_blank_payload_strings() {
        let text_request = InferenceExecutionRequest {
            request_id: Some("req-blank-text".to_string()),
            task_id: InferenceTaskId::TextGeneration,
            model_ref: None,
            model_name: Some("tiny".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::TextGeneration {
                prompt: Some("   ".to_string()),
                system_prompt: None,
                messages: Vec::new(),
                stream: false,
            },
            generation_options: None,
            extra_options: Value::Null,
        };
        assert!(matches!(
            text_request.validate(),
            Err(InferenceExecutionRequestValidationError::MissingTextInput)
        ));

        let embedding_request = InferenceExecutionRequest {
            request_id: Some("req-blank-embedding".to_string()),
            task_id: InferenceTaskId::Embedding,
            model_ref: None,
            model_name: Some("tiny".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::Embedding {
                texts: vec!["alpha".to_string(), " \t ".to_string()],
            },
            generation_options: None,
            extra_options: Value::Null,
        };
        match embedding_request.validate() {
            Err(InferenceExecutionRequestValidationError::BlankEmbeddingText { index }) => {
                assert_eq!(index, 1);
            }
            other => panic!("expected blank embedding text error, got {other:?}"),
        }

        let rerank_request = InferenceExecutionRequest {
            request_id: Some("req-blank-rerank".to_string()),
            task_id: InferenceTaskId::Rerank,
            model_ref: None,
            model_name: Some("tiny".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::Rerank {
                query: "find this".to_string(),
                documents: vec!["doc-a".to_string(), "\n".to_string()],
                top_n: None,
                return_documents: false,
            },
            generation_options: None,
            extra_options: Value::Null,
        };
        match rerank_request.validate() {
            Err(InferenceExecutionRequestValidationError::BlankRerankDocument { index }) => {
                assert_eq!(index, 1);
            }
            other => panic!("expected blank rerank document error, got {other:?}"),
        }
    }

    #[test]
    fn typed_execution_audio_transcription_validation_accepts_audio_refs() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-audio".to_string()),
            task_id: InferenceTaskId::AudioTranscription,
            model_ref: None,
            model_name: Some("tiny-audio".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::AudioTranscription {
                request: AudioTranscriptionRequest {
                    model: "tiny-audio".to_string(),
                    audio: None,
                    audio_ref: Some("artifact://audio.wav".to_string()),
                    language: None,
                    prompt: None,
                    task: None,
                    chunk_length_s: None,
                    extra_options: Value::Null,
                },
            },
            generation_options: None,
            extra_options: Value::Null,
        };

        request
            .validate()
            .expect("audio refs should satisfy typed ASR validation");
    }

    #[test]
    fn typed_execution_audio_transcription_validation_requires_audio_source() {
        let request = InferenceExecutionRequest {
            request_id: Some("req-audio-missing".to_string()),
            task_id: InferenceTaskId::AudioTranscription,
            model_ref: None,
            model_name: Some("tiny-audio".to_string()),
            runtime_hint: None,
            resolved_model_package_facts: None,
            input: InferenceExecutionInput::AudioTranscription {
                request: AudioTranscriptionRequest {
                    model: "tiny-audio".to_string(),
                    audio: None,
                    audio_ref: Some("  ".to_string()),
                    language: None,
                    prompt: None,
                    task: None,
                    chunk_length_s: None,
                    extra_options: Value::Null,
                },
            },
            generation_options: None,
            extra_options: Value::Null,
        };

        match request.validate() {
            Err(InferenceExecutionRequestValidationError::MissingAudioInput) => {}
            other => panic!("expected missing audio input error, got {other:?}"),
        }
    }

    #[test]
    fn typed_execution_rejects_registry_tasks_without_execution_contract() {
        for task_id in [
            InferenceTaskId::ImageUnderstanding,
            InferenceTaskId::DepthEstimation,
            InferenceTaskId::VideoUnderstanding,
            InferenceTaskId::MultimodalGeneration,
        ] {
            let request = InferenceExecutionRequest {
                request_id: Some(format!("req-{}", task_id.canonical_label())),
                task_id: task_id.clone(),
                model_ref: None,
                model_name: Some("roadmap-model".to_string()),
                runtime_hint: None,
                resolved_model_package_facts: None,
                input: InferenceExecutionInput::TextGeneration {
                    prompt: Some("hello".to_string()),
                    system_prompt: None,
                    messages: Vec::new(),
                    stream: false,
                },
                generation_options: None,
                extra_options: Value::Null,
            };

            match request.validate() {
                Err(InferenceExecutionRequestValidationError::UnsupportedTask {
                    task_id: rejected,
                }) => assert_eq!(rejected, task_id),
                other => panic!("expected unsupported task for {task_id:?}, got {other:?}"),
            }
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
        assert_eq!(
            decoded.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::TextGeneration
        );
        assert_eq!(decoded, result);
    }

    #[test]
    fn typed_embedding_result_serde_keeps_diagnostics_and_usage() {
        let result = InferenceExecutionResult::Embedding {
            embeddings: vec![InferenceEmbeddingResult {
                vector: vec![0.1, 0.2, 0.3],
                token_count: Some(3),
                index: Some(0),
            }],
            usage: Some(InferenceUsage {
                prompt_tokens: Some(3),
                completion_tokens: None,
                total_tokens: Some(3),
            }),
            option_diagnostics: vec![OptionCompatibilityDiagnostic {
                option_path: "extra_options.normalize".to_string(),
                state: crate::model_contracts::OptionSupportState::Mapped,
                backend_key: Some("candle".to_string()),
                message: Some("mapped to embedding backend options".to_string()),
            }],
        };

        let encoded = serde_json::to_value(&result).unwrap();
        let decoded: InferenceExecutionResult = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["result_type"], serde_json::json!("embedding"));
        assert_eq!(encoded["usage"]["prompt_tokens"], serde_json::json!(3));
        assert_eq!(
            encoded["embeddings"][0]["token_count"],
            serde_json::json!(3)
        );
        assert_eq!(
            encoded["option_diagnostics"][0]["option_path"],
            serde_json::json!("extra_options.normalize")
        );
        assert_eq!(
            decoded.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::Embedding
        );
        assert_eq!(decoded, result);
    }

    #[test]
    fn typed_rerank_result_serde_keeps_option_diagnostics() {
        let result = InferenceExecutionResult::Rerank {
            response: RerankResponse {
                results: vec![RerankResult {
                    index: 1,
                    score: 0.92,
                    document: Some("ranked document".to_string()),
                }],
                metadata: serde_json::json!({
                    "backend": "pytorch"
                }),
            },
            option_diagnostics: vec![OptionCompatibilityDiagnostic {
                option_path: "rerank.top_n".to_string(),
                state: crate::model_contracts::OptionSupportState::Honored,
                backend_key: Some("pytorch".to_string()),
                message: Some("passed through rerank request".to_string()),
            }],
        };

        let encoded = serde_json::to_value(&result).unwrap();
        let decoded: InferenceExecutionResult = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["result_type"], serde_json::json!("rerank"));
        assert_eq!(
            encoded["response"]["results"][0]["index"],
            serde_json::json!(1)
        );
        assert_eq!(
            encoded["option_diagnostics"][0]["state"],
            serde_json::json!("honored")
        );
        assert_eq!(
            decoded.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::Rerank
        );
        assert_eq!(decoded, result);
    }

    #[test]
    fn typed_image_generation_result_serde_keeps_option_diagnostics() {
        let result = InferenceExecutionResult::ImageGeneration {
            result: ImageGenerationResult {
                images: vec![EncodedImage {
                    data_base64: "aW1hZ2U=".to_string(),
                    mime_type: "image/png".to_string(),
                    width: Some(512),
                    height: Some(512),
                }],
                seed_used: Some(42),
                metadata: serde_json::json!({
                    "backend": "diffusers"
                }),
            },
            option_diagnostics: vec![OptionCompatibilityDiagnostic {
                option_path: "image.num_inference_steps".to_string(),
                state: crate::model_contracts::OptionSupportState::Honored,
                backend_key: Some("diffusers".to_string()),
                message: Some("forwarded to image generation backend".to_string()),
            }],
        };

        let encoded = serde_json::to_value(&result).unwrap();
        let decoded: InferenceExecutionResult = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(
            encoded["result_type"],
            serde_json::json!("image_generation")
        );
        assert_eq!(
            encoded["option_diagnostics"][0]["option_path"],
            serde_json::json!("image.num_inference_steps")
        );
        assert_eq!(
            encoded["option_diagnostics"][0]["state"],
            serde_json::json!("honored")
        );
        assert_eq!(
            decoded.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::ImageGeneration
        );
        assert_eq!(decoded, result);
    }

    #[test]
    fn typed_audio_result_serde_keeps_option_diagnostics() {
        let result = InferenceExecutionResult::AudioTranscription {
            result: AudioTranscriptionResult {
                text: "transcribed text".to_string(),
                language: Some("en".to_string()),
                duration_seconds: Some(1.0),
                segments: Vec::new(),
                metadata: Value::Null,
            },
            option_diagnostics: vec![OptionCompatibilityDiagnostic {
                option_path: "audio_transcription.language".to_string(),
                state: crate::model_contracts::OptionSupportState::Honored,
                backend_key: Some("mock".to_string()),
                message: Some("language hint forwarded".to_string()),
            }],
        };

        let encoded = serde_json::to_value(&result).unwrap();
        let decoded: InferenceExecutionResult = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(
            encoded["result_type"],
            serde_json::json!("audio_transcription")
        );
        assert_eq!(
            encoded["option_diagnostics"][0]["option_path"],
            serde_json::json!("audio_transcription.language")
        );
        assert_eq!(
            encoded["option_diagnostics"][0]["backend_key"],
            serde_json::json!("mock")
        );
        assert_eq!(
            decoded.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::AudioTranscription
        );
        assert_eq!(decoded, result);
    }

    #[test]
    fn typed_audio_result_option_diagnostics_accessor_matches_contract() {
        let result = InferenceExecutionResult::AudioTranscription {
            result: AudioTranscriptionResult {
                text: "transcribed text".to_string(),
                language: Some("en".to_string()),
                duration_seconds: Some(1.0),
                segments: Vec::new(),
                metadata: Value::Null,
            },
            option_diagnostics: vec![OptionCompatibilityDiagnostic {
                option_path: "audio_transcription.language".to_string(),
                state: crate::model_contracts::OptionSupportState::Honored,
                backend_key: Some("mock".to_string()),
                message: Some("language hint forwarded".to_string()),
            }],
        };

        assert_eq!(result.option_diagnostics().len(), 1);
        assert_eq!(
            result.option_diagnostics()[0].option_path,
            "audio_transcription.language"
        );
        assert_eq!(
            result.result_kind(),
            crate::model_contracts::InferenceExecutionResultKind::AudioTranscription
        );
    }

    #[test]
    fn typed_result_usage_and_cache_accessors_match_contract() {
        let text_result = InferenceExecutionResult::TextGeneration {
            text: "Done".to_string(),
            usage: Some(InferenceUsage {
                prompt_tokens: Some(8),
                completion_tokens: Some(5),
                total_tokens: Some(13),
            }),
            cache_handle_id: Some("kv-checkpoint-1".to_string()),
            option_diagnostics: Vec::new(),
        };
        let embedding_result = InferenceExecutionResult::Embedding {
            embeddings: Vec::new(),
            usage: Some(InferenceUsage {
                prompt_tokens: Some(3),
                completion_tokens: None,
                total_tokens: Some(3),
            }),
            option_diagnostics: Vec::new(),
        };
        let rerank_result = InferenceExecutionResult::Rerank {
            response: RerankResponse {
                results: Vec::new(),
                metadata: Value::Null,
            },
            option_diagnostics: Vec::new(),
        };

        assert_eq!(
            text_result.usage().and_then(|usage| usage.total_tokens),
            Some(13)
        );
        assert_eq!(text_result.cache_handle_id(), Some("kv-checkpoint-1"));
        assert_eq!(
            embedding_result
                .usage()
                .and_then(|usage| usage.prompt_tokens),
            Some(3)
        );
        assert_eq!(embedding_result.cache_handle_id(), None);
        assert!(rerank_result.usage().is_none());
        assert_eq!(rerank_result.cache_handle_id(), None);
    }

    #[test]
    fn inference_request_lifecycle_event_serde_uses_stable_contract() {
        let event = InferenceRequestLifecycleEvent {
            request_id: Some("req-1".to_string()),
            phase: InferenceLifecyclePhase::BackendExecution,
            kind: InferenceRequestLifecycleEventKind::CleanupCompleted,
            occurred_at_ms: 42,
            task_id: Some("text_generation".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-1".to_string()),
            selected_device_id: Some("CUDA0".to_string()),
            selected_network_node_id: Some("local-node-alpha".to_string()),
            model_id: Some("pumas://models/tiny-llama".to_string()),
            resolved_artifact_kind: Some("gguf".to_string()),
            usage: Some(InferenceUsage {
                prompt_tokens: Some(8),
                completion_tokens: Some(5),
                total_tokens: Some(13),
            }),
            cache_handle_id: Some("kv-checkpoint-1".to_string()),
            artifact_refs: vec!["artifact://audio.wav".to_string()],
            detail: Some("stream dropped by consumer".to_string()),
            canonical_error_event_id: Some("diagnostic-error-inference-1".to_string()),
            compatibility_report: Some(InferenceCompatibilityReportSummary {
                status: "accepted".to_string(),
                compatible: true,
                task: "supported".to_string(),
                model_source: "supported".to_string(),
                preprocessing: "supported".to_string(),
                postprocessing: "supported".to_string(),
            }),
            compatibility_issues: vec![InferenceCompatibilityIssueSummary {
                kind: "unsupported_option".to_string(),
                phase: InferenceLifecyclePhase::TaskValidation,
                message: "sampling option was ignored".to_string(),
                model_id: Some("pumas://models/tiny-llama".to_string()),
                path: Some("sampling.temperature".to_string()),
            }],
            option_diagnostics: Vec::new(),
        };

        let encoded = serde_json::to_value(&event).unwrap();
        let decoded: InferenceRequestLifecycleEvent =
            serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(encoded["phase"], serde_json::json!("backend_execution"));
        assert_eq!(encoded["kind"], serde_json::json!("cleanup_completed"));
        assert_eq!(
            encoded["model_id"],
            serde_json::json!("pumas://models/tiny-llama")
        );
        assert_eq!(encoded["resolved_artifact_kind"], serde_json::json!("gguf"));
        assert_eq!(encoded["runtime_id"], serde_json::json!("llama.cpp"));
        assert_eq!(encoded["selected_device_id"], serde_json::json!("CUDA0"));
        assert_eq!(
            encoded["selected_network_node_id"],
            serde_json::json!("local-node-alpha")
        );
        assert_eq!(encoded["task_id"], serde_json::json!("text_generation"));
        assert_eq!(encoded["usage"]["total_tokens"], serde_json::json!(13));
        assert_eq!(
            encoded["cache_handle_id"],
            serde_json::json!("kv-checkpoint-1")
        );
        assert_eq!(
            encoded["artifact_refs"],
            serde_json::json!(["artifact://audio.wav"])
        );
        assert_eq!(
            encoded["canonical_error_event_id"],
            serde_json::json!("diagnostic-error-inference-1")
        );
        assert_eq!(
            encoded["compatibility_report"]["compatible"],
            serde_json::json!(true)
        );
        assert_eq!(
            encoded["compatibility_issues"][0]["kind"],
            serde_json::json!("unsupported_option")
        );
        assert_eq!(decoded, event);
    }

    #[test]
    fn bounded_inference_artifact_ref_filters_local_path_shapes() {
        assert_eq!(
            bounded_inference_artifact_ref(" artifact://audio.wav ").as_deref(),
            Some("artifact://audio.wav")
        );
        assert_eq!(
            bounded_inference_artifact_ref("https://cdn.example/audio.wav").as_deref(),
            Some("https://cdn.example/audio.wav")
        );

        for value in [
            "",
            "/tmp/private.wav",
            "./private.wav",
            "../private.wav",
            "~/private.wav",
            "file:///tmp/private.wav",
            "C:\\Users\\jeremy\\private.wav",
            "D:/Users/jeremy/private.wav",
        ] {
            assert_eq!(bounded_inference_artifact_ref(value), None, "{value}");
            if !value.is_empty() {
                assert!(looks_like_local_artifact_ref(value), "{value}");
            }
        }
    }

    #[test]
    fn inference_request_lifecycle_event_serde_defaults_and_ignores_unknown_fields() {
        let encoded = serde_json::json!({
            "phase": "task_validation",
            "kind": "completed",
            "occurred_at_ms": 99,
            "producer_future_field": {
                "ignored": true
            },
            "compatibility_issues_future": [
                "ignored"
            ]
        });

        let decoded: InferenceRequestLifecycleEvent =
            serde_json::from_value(encoded).expect("minimal lifecycle event decodes");

        assert_eq!(decoded.request_id, None);
        assert_eq!(decoded.phase, InferenceLifecyclePhase::TaskValidation);
        assert_eq!(decoded.kind, InferenceRequestLifecycleEventKind::Completed);
        assert_eq!(decoded.occurred_at_ms, 99);
        assert_eq!(decoded.task_id, None);
        assert_eq!(decoded.backend_key, None);
        assert_eq!(decoded.runtime_id, None);
        assert_eq!(decoded.runtime_instance_id, None);
        assert_eq!(decoded.selected_device_id, None);
        assert_eq!(decoded.selected_network_node_id, None);
        assert_eq!(decoded.model_id, None);
        assert_eq!(decoded.resolved_artifact_kind, None);
        assert_eq!(decoded.usage, None);
        assert_eq!(decoded.cache_handle_id, None);
        assert_eq!(decoded.detail, None);
        assert_eq!(decoded.canonical_error_event_id, None);
        assert_eq!(decoded.compatibility_report, None);
        assert!(decoded.compatibility_issues.is_empty());
        assert!(decoded.option_diagnostics.is_empty());
    }

    #[test]
    fn inference_request_lifecycle_sink_error_serde_uses_stable_contract() {
        let error = InferenceRequestLifecycleEventSinkError::diagnostics_unavailable(
            "failed to record inference lifecycle diagnostic",
        );

        let encoded = serde_json::to_value(&error).expect("sink error encodes");
        let decoded: InferenceRequestLifecycleEventSinkError =
            serde_json::from_value(encoded.clone()).expect("sink error decodes");

        assert_eq!(
            encoded["code"],
            serde_json::json!("diagnostics_unavailable")
        );
        assert_eq!(
            encoded["message"],
            serde_json::json!("failed to record inference lifecycle diagnostic")
        );
        assert_eq!(decoded, error);
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
            active_resolved_device: Some("cuda:0".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            embedding_resolved_device: Some("cpu".to_string()),
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
        assert_eq!(facts[0].resolved_device.as_deref(), Some("cuda:0"));
        assert_eq!(facts[0].readiness, RuntimeFactReadiness::Ready);
        assert_eq!(
            facts[1].active_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert_eq!(facts[1].resolved_device.as_deref(), Some("cpu"));
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
            active_resolved_device: None,
            embedding_model_target: None,
            embedding_resolved_device: None,
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
