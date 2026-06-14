use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::backend::BackendError;
use crate::device_contracts::InferenceDeviceId;
use crate::model_contracts::{
    InferenceTaskId, ModelArtifactKind, ModelAuthTokenSource, ModelLoadCachePolicy,
    ModelLoadNetworkPolicy, ModelLoadSecurityPolicy, ModelRemoteCodePolicy,
    OptionCompatibilityDiagnostic, ProcessorComponentKind, PumasModelRef, ResolvedModelSource,
};
use crate::resource_observation::InferenceExecutionResourceObservation;

pub(super) const PYTORCH_WORKER_CONTRACT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub(super) struct PyTorchWorkerEnvelope<T> {
    pub contract_version: u32,
    pub request_id: String,
    pub operation: PyTorchWorkerOperation,
    #[serde(default)]
    pub cancellation: PyTorchWorkerCancellation,
    pub payload: T,
}

impl<T> PyTorchWorkerEnvelope<T> {
    #[must_use]
    pub(super) fn new(
        request_id: impl Into<String>,
        operation: PyTorchWorkerOperation,
        payload: T,
    ) -> Self {
        Self {
            contract_version: PYTORCH_WORKER_CONTRACT_VERSION,
            request_id: request_id.into(),
            operation,
            cancellation: PyTorchWorkerCancellation::default(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum PyTorchWorkerOperation {
    InitWorker,
    ShutdownWorker,
    GetLoadedInfo,
    LoadTransformersModel,
    UnloadModel,
    GenerateText,
    GenerateTextStream,
    GenerateImage,
    GenerateImageBatch,
    TranscribeAudio,
    SaveKvCache,
    RestoreKvCache,
    ClearKvCache,
    TruncateKvCache,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchWorkerCancellation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default)]
    pub drop_stream_cancels: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchTransformersLoadRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_ref: Option<PumasModelRef>,
    pub artifact_kind: ModelArtifactKind,
    pub entry_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_source: Option<ResolvedModelSource>,
    pub task_id: InferenceTaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_profile: Option<PyTorchTransformersTaskProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_type_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<InferenceDeviceId>,
    #[serde(default)]
    pub trust_policy: PyTorchTransformersTrustPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_defaults: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchTransformersTaskProfile {
    pub task_id: InferenceTaskId,
    pub canonical_task_label: String,
    pub loader: PyTorchTransformersModelLoader,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_components: Vec<ProcessorComponentKind>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum PyTorchTransformersModelLoader {
    CausalLm,
    AutomaticSpeechRecognition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchTransformersTrustPolicy {
    #[serde(default)]
    pub allow_remote_code: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_id: Option<String>,
    #[serde(default)]
    pub local_files_only: bool,
    #[serde(default)]
    pub cache_policy: ModelLoadCachePolicy,
    #[serde(default)]
    pub auth_token_source: ModelAuthTokenSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_revision: Option<String>,
}

impl Default for PyTorchTransformersTrustPolicy {
    fn default() -> Self {
        ModelLoadSecurityPolicy::default().into()
    }
}

impl From<ModelLoadSecurityPolicy> for PyTorchTransformersTrustPolicy {
    fn from(policy: ModelLoadSecurityPolicy) -> Self {
        Self {
            allow_remote_code: policy.trust_remote_code == ModelRemoteCodePolicy::Allow,
            accepted_sources: policy.accepted_code_sources,
            decision_id: policy.decision_id,
            local_files_only: policy.network == ModelLoadNetworkPolicy::LocalOnly,
            cache_policy: policy.cache,
            auth_token_source: policy.auth_token_source,
            revision: policy.revision,
            code_revision: policy.code_revision,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchGenerateTextRequest {
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    pub max_tokens: i64,
    pub temperature: f64,
    pub top_p: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub masked_prompt_json: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denoising_steps: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_length: Option<i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub transformers_kwargs: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchGenerateTextResult {
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchUnloadModelRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchUnloadModelResult {
    pub unloaded: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchInitWorkerRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchInitWorkerResult {
    pub initialized: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchShutdownWorkerRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchShutdownWorkerResult {
    pub shutdown: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchGetLoadedInfoRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchClearKvCacheRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchClearKvCacheResult {
    pub cleared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchSaveKvCacheRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchRestoreKvCacheRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchTruncateKvCacheRequest {
    pub path: String,
    pub token_position: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchTruncateKvCacheResult {
    pub token_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchAudioTranscriptionRequest {
    pub model_path: String,
    pub audio_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<InferenceDeviceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_length_s: Option<f32>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub extra_options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchAudioTranscriptionResult {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "status")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub(super) enum PyTorchWorkerResponse<T> {
    Ok(PyTorchWorkerSuccess<T>),
    Error(PyTorchWorkerFailure),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub(super) struct PyTorchWorkerSuccess<T> {
    pub request_id: String,
    pub result: T,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_observation: Option<InferenceExecutionResourceObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchWorkerFailure {
    pub request_id: String,
    pub error: PyTorchWorkerError,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_observation: Option<InferenceExecutionResourceObservation>,
}

#[allow(dead_code)]
impl PyTorchWorkerFailure {
    pub(super) fn into_backend_error(self) -> BackendError {
        let message = self.error.backend_message(&self.request_id);
        match self.error.kind {
            PyTorchWorkerErrorKind::InvalidRequest
            | PyTorchWorkerErrorKind::UnsupportedTask
            | PyTorchWorkerErrorKind::TrustPolicyRejected => BackendError::Config(message),
            PyTorchWorkerErrorKind::RuntimeUnavailable => BackendError::NotRunning(message),
            PyTorchWorkerErrorKind::ModelLoadFailed => BackendError::StartupFailed(message),
            PyTorchWorkerErrorKind::GenerationFailed
            | PyTorchWorkerErrorKind::Cancelled
            | PyTorchWorkerErrorKind::Internal => BackendError::Inference(message),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) struct PyTorchWorkerError {
    pub kind: PyTorchWorkerErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_code: Option<String>,
}

impl PyTorchWorkerError {
    fn backend_message(&self, request_id: &str) -> String {
        let message = normalize_worker_error_message(&self.message, "Python worker failed");
        match self.canonical_code.as_deref() {
            Some(code) => format!("PyTorch worker {code} for {request_id}: {message}"),
            None => format!("PyTorch worker error for {request_id}: {message}"),
        }
    }
}

pub(super) fn normalize_worker_error_message(message: &str, fallback: &str) -> String {
    const MAX_PARTS: usize = 6;
    const MAX_CHARS: usize = 512;

    let mut parts = Vec::new();
    for line in message.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || is_python_traceback_frame(line, trimmed) {
            continue;
        }
        parts.push(redact_local_path_tokens(trimmed));
        if parts.len() >= MAX_PARTS {
            break;
        }
    }

    let normalized = if parts.is_empty() {
        fallback.to_string()
    } else {
        parts.join(" | ")
    };
    truncate_chars(normalized, MAX_CHARS)
}

fn is_python_traceback_frame(raw_line: &str, trimmed: &str) -> bool {
    raw_line.starts_with(' ')
        || raw_line.starts_with('\t')
        || trimmed.starts_with("Traceback (most recent call last):")
        || trimmed.starts_with("File \"")
        || trimmed.starts_with("File '")
        || trimmed.starts_with('^')
        || trimmed.starts_with("During handling of the above exception")
        || trimmed.starts_with("The above exception was the direct cause")
}

fn redact_local_path_tokens(line: &str) -> String {
    line.split_whitespace()
        .map(|token| {
            if token_contains_local_path(token) {
                "[local-path]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn token_contains_local_path(token: &str) -> bool {
    let trimmed = token.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | '`' | ',' | ':' | ';' | ')' | '(' | '[' | ']' | '{' | '}'
        )
    });
    trimmed.starts_with('/')
        || trimmed.starts_with("~/")
        || trimmed.starts_with("file://")
        || trimmed.contains("\\\\")
        || (trimmed.len() > 2
            && trimmed.as_bytes()[1] == b':'
            && trimmed.as_bytes()[0].is_ascii_alphabetic()
            && (trimmed.as_bytes()[2] == b'\\' || trimmed.as_bytes()[2] == b'/'))
}

fn truncate_chars(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }

    let mut truncated = value.chars().take(max_chars).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum PyTorchWorkerErrorKind {
    InvalidRequest,
    UnsupportedTask,
    TrustPolicyRejected,
    ModelLoadFailed,
    GenerationFailed,
    Cancelled,
    RuntimeUnavailable,
    Internal,
}
