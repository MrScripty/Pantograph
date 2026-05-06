//! PyTorch backend implementation (in-process via PyO3)
//!
//! Embeds a Python interpreter to run PyTorch inference directly in the
//! Pantograph process. Supports HuggingFace models, dLLMs (e.g., TraDo),
//! and Sherry ternary quantized models.
//!
//! The Python worker module (`torch/worker.py`) is embedded at compile time
//! via `include_str!` and loaded into `sys.modules` on first use.

use std::collections::BTreeMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use self::pytorch_worker_contract::{
    normalize_worker_error_message, PyTorchAudioTranscriptionRequest,
    PyTorchAudioTranscriptionResult, PyTorchGenerateTextRequest, PyTorchGenerateTextResult,
    PyTorchTransformersLoadRequest, PyTorchTransformersModelLoader, PyTorchTransformersTaskProfile,
    PyTorchTransformersTrustPolicy, PyTorchWorkerEnvelope, PyTorchWorkerError,
    PyTorchWorkerErrorKind, PyTorchWorkerFailure, PyTorchWorkerOperation, PyTorchWorkerResponse,
    PYTORCH_WORKER_CONTRACT_VERSION,
};
use super::{
    BackendCapabilities, BackendCapabilityFacts, BackendComponentCapability, BackendConfig,
    BackendError, BackendFeatureCapabilityFacts, BackendFeatureSupport,
    BackendModelSourceCapabilityFacts, BackendStartOutcome, BackendTaskCapability, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::model_contracts::{
    resolve_task_registry_entry_from_evidence, GenerationOptions, InferenceModality,
    InferenceTaskId, ModelLoadSecurityPolicy, ModelValidationState, OptionCompatibilityDiagnostic,
    OptionSupportState, ResolvedModelPackageFacts, ResolvedModelSource, ResolvedModelSourceKind,
    TaskEvidence, TaskRegistryEntry,
};
use crate::process::ProcessSpawner;
use crate::types::{
    AudioTranscriptionRequest, AudioTranscriptionResult, RerankRequest, RerankResponse,
};
use crate::{BackendHintLabel, ModelArtifactKind};
use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};

#[path = "pytorch_worker.rs"]
mod pytorch_worker;
#[allow(dead_code)]
#[path = "pytorch_worker_contract.rs"]
mod pytorch_worker_contract;

const ALLOWED_TRANSFORMERS_GENERATE_KWARGS: &[&str] = &["top_k"];

/// PyTorch backend using in-process PyO3 embedded Python.
///
/// Loads models via HuggingFace transformers with `trust_remote_code=True`,
/// supporting standard models, dLLM architectures, and Sherry quantised models.
pub struct PyTorchBackend {
    /// Whether the backend has been initialised and is ready
    ready: bool,
    /// Currently loaded model metadata
    loaded_model: Option<LoadedModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadedModelInfo {
    pub model_path: String,
    pub model_type: String,
    pub device: String,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyTorchTransformersLoadArgs {
    model_path: String,
    device: String,
    model_type: Option<String>,
    trust_policy: PyTorchTransformersTrustPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyTorchLiveKvInfo {
    pub token_count: usize,
    pub model_path: String,
    pub model_type: String,
    pub device: String,
}

#[derive(Debug, Clone, PartialEq)]
struct PyTorchGenerationOptionMapping {
    kwargs: Map<String, Value>,
    diagnostics: Vec<OptionCompatibilityDiagnostic>,
}

pub fn kv_cache_runtime_fingerprint_for_live_kv(
    info: &PyTorchLiveKvInfo,
) -> KvCacheRuntimeFingerprint {
    kv_cache_runtime_fingerprint_for_loaded_model(&LoadedModelInfo {
        model_path: info.model_path.clone(),
        model_type: info.model_type.clone(),
        device: info.device.clone(),
    })
}

pub fn kv_cache_model_fingerprint_for_live_kv(info: &PyTorchLiveKvInfo) -> ModelFingerprint {
    kv_cache_model_fingerprint_for_loaded_model(&LoadedModelInfo {
        model_path: info.model_path.clone(),
        model_type: info.model_type.clone(),
        device: info.device.clone(),
    })
}

pub fn kv_cache_runtime_fingerprint_for_loaded_model(
    loaded: &LoadedModelInfo,
) -> KvCacheRuntimeFingerprint {
    PyTorchBackend::kv_cache_runtime_fingerprint_for_loaded_model(loaded)
}

pub fn kv_cache_model_fingerprint_for_loaded_model(loaded: &LoadedModelInfo) -> ModelFingerprint {
    PyTorchBackend::kv_cache_model_fingerprint_for_loaded_model(loaded)
}

pub fn supports_live_kv_reuse(model_type: &str) -> bool {
    model_type == "dllm"
}

fn kv_worker_failure_from_message(
    request_id: &str,
    canonical_code: &'static str,
    message: String,
) -> BackendError {
    PyTorchWorkerFailure {
        request_id: request_id.to_string(),
        error: PyTorchWorkerError {
            kind: PyTorchWorkerErrorKind::GenerationFailed,
            message: normalize_worker_error_message(&message, "Python worker transport failed"),
            canonical_code: Some(canonical_code.to_string()),
        },
    }
    .into_backend_error()
}

fn task_join_error_message(error: impl std::fmt::Display) -> String {
    normalize_worker_error_message(
        &format!("Task join error: {error}"),
        "PyTorch worker task join failed",
    )
}

pub async fn active_loaded_model_info() -> Result<LoadedModelInfo, BackendError> {
    let request_id = format!("pytorch-kv-loaded-info-{}", Uuid::new_v4().simple());
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<LoadedModelInfo, BackendError> {
            let worker = pytorch_worker::worker_module(py).map_err(|e| {
                kv_worker_failure_from_message(
                    &request_id,
                    "pytorch_worker_kv_loaded_info_failed",
                    format!("Failed to get worker module: {}", e),
                )
            })?;
            let result = worker.call_method0("get_loaded_info").map_err(|e| {
                kv_worker_failure_from_message(
                    &request_id,
                    "pytorch_worker_kv_loaded_info_failed",
                    format!("PyTorch get_loaded_info failed: {}", e),
                )
            })?;
            if result.is_none() {
                return Err(BackendError::Inference(
                    "PyTorch KV operations require an active loaded model".to_string(),
                ));
            }
            pytorch_worker::extract_loaded_model_info(&result)
        })
    })
    .await
    .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
}

pub async fn save_live_kv_snapshot(path: &Path) -> Result<PyTorchLiveKvInfo, BackendError> {
    let path = path.to_path_buf();
    let request_id = format!("pytorch-kv-save-{}", Uuid::new_v4().simple());
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<PyTorchLiveKvInfo, BackendError> {
            let worker = pytorch_worker::worker_module(py).map_err(|e| {
                kv_worker_failure_from_message(
                    &request_id,
                    "pytorch_worker_kv_save_failed",
                    format!("Failed to get worker module: {}", e),
                )
            })?;
            let result = worker
                .call_method1("save_live_kv_cache", (path.to_string_lossy().to_string(),))
                .map_err(|e| {
                    kv_worker_failure_from_message(
                        &request_id,
                        "pytorch_worker_kv_save_failed",
                        format!("PyTorch KV save failed: {}", e),
                    )
                })?;
            pytorch_worker::extract_live_kv_info(&result)
        })
    })
    .await
    .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
}

pub async fn restore_live_kv_snapshot(path: &Path) -> Result<PyTorchLiveKvInfo, BackendError> {
    let path = path.to_path_buf();
    let request_id = format!("pytorch-kv-restore-{}", Uuid::new_v4().simple());
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<PyTorchLiveKvInfo, BackendError> {
            let worker = pytorch_worker::worker_module(py).map_err(|e| {
                kv_worker_failure_from_message(
                    &request_id,
                    "pytorch_worker_kv_restore_failed",
                    format!("Failed to get worker module: {}", e),
                )
            })?;
            let result = worker
                .call_method1(
                    "restore_live_kv_cache",
                    (path.to_string_lossy().to_string(),),
                )
                .map_err(|e| {
                    kv_worker_failure_from_message(
                        &request_id,
                        "pytorch_worker_kv_restore_failed",
                        format!("PyTorch KV restore failed: {}", e),
                    )
                })?;
            pytorch_worker::extract_live_kv_info(&result)
        })
    })
    .await
    .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
}

pub async fn clear_live_kv_snapshot() -> Result<(), BackendError> {
    let request_id = format!("pytorch-kv-clear-{}", Uuid::new_v4().simple());
    tokio::task::spawn_blocking(move || {
        Python::with_gil(|py| -> Result<(), BackendError> {
            let worker = pytorch_worker::worker_module(py).map_err(|e| {
                kv_worker_failure_from_message(
                    &request_id,
                    "pytorch_worker_kv_clear_failed",
                    format!("Failed to get worker module: {}", e),
                )
            })?;
            worker.call_method0("clear_live_kv_cache").map_err(|e| {
                kv_worker_failure_from_message(
                    &request_id,
                    "pytorch_worker_kv_clear_failed",
                    format!("PyTorch KV clear failed: {}", e),
                )
            })?;
            Ok(())
        })
    })
    .await
    .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
}

impl PyTorchBackend {
    pub fn new() -> Self {
        Self {
            ready: false,
            loaded_model: None,
        }
    }

    /// Get static capabilities (for registry info before instantiation)
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            vision: false,
            image_generation: false,
            embeddings: false,
            reranking: false,
            gpu: true,
            device_selection: true,
            streaming: true,
            tool_calling: false,
            external_connection: false,
            facts: BackendCapabilityFacts {
                tasks: vec![
                    BackendTaskCapability::stable(
                        InferenceTaskId::TextGeneration,
                        vec![InferenceModality::Text],
                        vec![InferenceModality::Text],
                    ),
                    BackendTaskCapability::stable(
                        InferenceTaskId::AudioTranscription,
                        vec![InferenceModality::Audio],
                        vec![InferenceModality::Text],
                    ),
                ],
                preprocessing: BackendComponentCapability::RequiresPackageComponent,
                postprocessing: BackendComponentCapability::BackendManaged,
                model_sources: BackendModelSourceCapabilityFacts {
                    artifact_kinds: vec![
                        ModelArtifactKind::HfCompatibleDirectory,
                        ModelArtifactKind::Safetensors,
                    ],
                    backend_hints: vec![BackendHintLabel::Transformers],
                    custom_code: BackendFeatureSupport::Supported,
                },
                features: BackendFeatureCapabilityFacts {
                    streaming: BackendFeatureSupport::Supported,
                    device_selection: BackendFeatureSupport::Supported,
                    external_connection: BackendFeatureSupport::Unsupported,
                    kv_cache: BackendFeatureSupport::Supported,
                },
            },
        }
    }

    /// Check if Python 3 is available on the system
    pub fn check_availability() -> (bool, Option<String>) {
        match which::which("python3") {
            Ok(_) => (true, None),
            Err(_) => (
                false,
                Some("python3 not found in PATH. Install Python 3 with PyTorch.".to_string()),
            ),
        }
    }

    fn can_reuse_loaded_model(
        &self,
        model_path: &str,
        device: &str,
        model_type: Option<&str>,
    ) -> bool {
        self.loaded_model.as_ref().is_some_and(|loaded| {
            loaded.model_path == model_path
                && loaded.device == device
                && model_type.is_none_or(|requested| loaded.model_type == requested)
        })
    }

    fn active_loaded_model(&self) -> Result<&LoadedModelInfo, BackendError> {
        self.loaded_model.as_ref().ok_or_else(|| {
            BackendError::Inference(
                "KV cache operations require an active loaded PyTorch model".to_string(),
            )
        })
    }

    fn kv_cache_runtime_fingerprint_for_loaded_model(
        loaded: &LoadedModelInfo,
    ) -> KvCacheRuntimeFingerprint {
        KvCacheRuntimeFingerprint {
            runtime_id: canonical_runtime_id("pytorch"),
            backend_key: canonical_runtime_backend_key("pytorch"),
            tokenizer_fingerprint: format!("pytorch:{}:{}", loaded.model_path, loaded.model_type),
            prompt_format_fingerprint: Some(format!("pytorch_{}", loaded.model_type)),
            runtime_build_fingerprint: Some(loaded.device.clone()),
        }
    }

    fn kv_cache_model_fingerprint_for_loaded_model(loaded: &LoadedModelInfo) -> ModelFingerprint {
        ModelFingerprint {
            model_id: loaded.model_path.clone(),
            config_hash: format!("pytorch:{}", loaded.model_type),
        }
    }

    fn require_live_kv_slot(slot_id: u32) -> Result<(), BackendError> {
        if slot_id == 0 {
            return Ok(());
        }
        Err(BackendError::Config(
            "PyTorch backend exposes only a single live KV slot at slot_id 0".to_string(),
        ))
    }

    /// Load a model into the embedded Python runtime.
    pub async fn load_model(
        &mut self,
        model_path: &str,
        device: &str,
        model_type: Option<&str>,
    ) -> Result<LoadedModelInfo, BackendError> {
        self.load_model_with_trust_policy(
            model_path,
            device,
            model_type,
            Self::default_transformers_trust_policy(),
        )
        .await
    }

    /// Load a Pumas-resolved Transformers package through the worker envelope
    /// contract before entering Python.
    pub async fn load_transformers_package(
        &mut self,
        request_id: impl Into<String>,
        package: &ResolvedModelPackageFacts,
        device: Option<&str>,
        security_policy: ModelLoadSecurityPolicy,
    ) -> Result<LoadedModelInfo, BackendError> {
        let envelope = Self::transformers_load_envelope_from_package(
            request_id,
            package,
            device,
            PyTorchTransformersTrustPolicy::from(security_policy),
        )?;
        self.load_transformers_envelope(envelope).await
    }

    fn default_transformers_trust_policy() -> PyTorchTransformersTrustPolicy {
        PyTorchTransformersTrustPolicy::default()
    }

    fn transformers_load_envelope_from_package(
        request_id: impl Into<String>,
        package: &ResolvedModelPackageFacts,
        device: Option<&str>,
        trust_policy: PyTorchTransformersTrustPolicy,
    ) -> Result<PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest>, BackendError> {
        if !package.uses_current_contract() {
            return Err(BackendError::Config(format!(
                "PyTorch/Transformers package facts contract version {} is unsupported",
                package.package_facts_contract_version
            )));
        }
        if matches!(
            package.artifact.validation_state,
            ModelValidationState::Invalid | ModelValidationState::Unknown
        ) {
            return Err(BackendError::Config(
                "PyTorch/Transformers package artifact is not valid".to_string(),
            ));
        }
        if !matches!(
            package.artifact.artifact_kind,
            ModelArtifactKind::HfCompatibleDirectory | ModelArtifactKind::Safetensors
        ) {
            return Err(BackendError::Config(format!(
                "PyTorch/Transformers cannot load {:?} artifacts",
                package.artifact.artifact_kind
            )));
        }

        let task_profile = Self::transformers_task_profile_from_evidence(&package.task)?;
        let task_id = task_profile.task_id.clone();
        if package.custom_code.requires_custom_code && !trust_policy.allow_remote_code {
            return Err(BackendError::Config(
                "Model package requires custom Transformers code but trust policy is closed"
                    .to_string(),
            ));
        }

        let model_type_hint = package
            .transformers
            .as_ref()
            .and_then(|facts| facts.config_model_type.clone());
        let generation_defaults = package.generation_defaults.defaults.clone();

        Ok(PyTorchWorkerEnvelope::new(
            request_id,
            PyTorchWorkerOperation::LoadTransformersModel,
            PyTorchTransformersLoadRequest {
                model_ref: Some(package.model_ref.clone()),
                artifact_kind: package.artifact.artifact_kind.clone(),
                entry_path: package.artifact.entry_path.clone(),
                model_source: Some(ResolvedModelSource::from_package_facts(package)),
                task_id,
                task_profile: Some(task_profile),
                model_type_hint,
                device: device.map(str::to_string),
                trust_policy,
                generation_defaults,
            },
        ))
    }

    fn transformers_load_envelope_from_direct_path(
        request_id: impl Into<String>,
        model_path: impl Into<String>,
        device: Option<&str>,
        model_type: Option<&str>,
        trust_policy: PyTorchTransformersTrustPolicy,
    ) -> PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest> {
        let model_path = model_path.into();
        PyTorchWorkerEnvelope::new(
            request_id,
            PyTorchWorkerOperation::LoadTransformersModel,
            PyTorchTransformersLoadRequest {
                model_ref: None,
                artifact_kind: ModelArtifactKind::HfCompatibleDirectory,
                entry_path: model_path.clone(),
                model_source: Some(ResolvedModelSource::direct_local(
                    ResolvedModelSourceKind::DirectHfCompatibleDirectory,
                    ModelArtifactKind::HfCompatibleDirectory,
                    model_path,
                )),
                task_id: InferenceTaskId::TextGeneration,
                task_profile: Some(PyTorchTransformersTaskProfile {
                    task_id: InferenceTaskId::TextGeneration,
                    canonical_task_label: "text_generation".to_string(),
                    loader: PyTorchTransformersModelLoader::CausalLm,
                    required_components: vec![],
                }),
                model_type_hint: model_type.map(str::to_string),
                device: device.map(str::to_string),
                trust_policy,
                generation_defaults: None,
            },
        )
    }

    async fn load_transformers_envelope(
        &mut self,
        envelope: PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest>,
    ) -> Result<LoadedModelInfo, BackendError> {
        Self::validate_transformers_load_envelope(&envelope)?;
        let request_id = envelope.request_id.clone();
        let envelope_json = serde_json::to_string(&envelope).map_err(|error| {
            BackendError::Config(format!(
                "Failed to encode PyTorch worker load envelope: {error}"
            ))
        })?;

        let info = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<LoadedModelInfo, BackendError> {
                let worker = pytorch_worker::worker_module(py).map_err(|e| {
                    Self::load_worker_failure_from_message(
                        &request_id,
                        format!("Failed to load worker module: {}", e),
                    )
                })?;

                let response_json = worker
                    .call_method1("load_transformers_model_from_envelope", (envelope_json,))
                    .and_then(|result| result.extract::<String>())
                    .map_err(|e| {
                        Self::load_worker_failure_from_message(
                            &request_id,
                            format!("Transformers envelope model load failed: {}", e),
                        )
                    })?;

                Self::load_info_from_worker_response(&request_id, &response_json)
            })
        })
        .await
        .map_err(|e| BackendError::Inference(task_join_error_message(e)))??;

        self.loaded_model = Some(info.clone());
        self.ready = true;
        Ok(info)
    }

    fn load_info_from_worker_response(
        request_id: &str,
        response_json: &str,
    ) -> Result<LoadedModelInfo, BackendError> {
        let response: PyTorchWorkerResponse<LoadedModelInfo> = serde_json::from_str(response_json)
            .map_err(|error| {
                Self::load_worker_failure_from_message(
                    request_id,
                    format!("Failed to decode PyTorch worker load response: {error}"),
                )
            })?;
        match response {
            PyTorchWorkerResponse::Ok(success) => {
                if success.request_id != request_id {
                    return Err(Self::load_worker_failure_from_message(
                        request_id,
                        format!(
                            "PyTorch worker load response request_id mismatch: expected {request_id}, got {}",
                            success.request_id
                        ),
                    ));
                }
                Ok(success.result)
            }
            PyTorchWorkerResponse::Error(failure) => {
                if failure.request_id != request_id {
                    return Err(Self::load_worker_failure_from_message(
                        request_id,
                        format!(
                            "PyTorch worker load response request_id mismatch: expected {request_id}, got {}",
                            failure.request_id
                        ),
                    ));
                }
                Err(failure.into_backend_error())
            }
        }
    }

    fn stream_setup_from_worker_response(
        request_id: &str,
        response_json: &str,
    ) -> Result<(), BackendError> {
        let response: PyTorchWorkerResponse<Value> =
            serde_json::from_str(response_json).map_err(|error| {
                Self::stream_worker_failure_from_message(
                    request_id,
                    format!("Failed to decode PyTorch worker stream setup response: {error}"),
                )
            })?;
        match response {
            PyTorchWorkerResponse::Ok(success) => {
                if success.request_id != request_id {
                    return Err(Self::stream_worker_failure_from_message(
                        request_id,
                        format!(
                            "PyTorch worker stream setup response request_id mismatch: expected {request_id}, got {}",
                            success.request_id
                        ),
                    ));
                }
                Ok(())
            }
            PyTorchWorkerResponse::Error(failure) => {
                if failure.request_id != request_id {
                    return Err(Self::stream_worker_failure_from_message(
                        request_id,
                        format!(
                            "PyTorch worker stream setup response request_id mismatch: expected {request_id}, got {}",
                            failure.request_id
                        ),
                    ));
                }
                Err(failure.into_backend_error())
            }
        }
    }

    fn load_worker_failure_from_message(request_id: &str, message: String) -> BackendError {
        PyTorchWorkerFailure {
            request_id: request_id.to_string(),
            error: PyTorchWorkerError {
                kind: PyTorchWorkerErrorKind::ModelLoadFailed,
                message: normalize_worker_error_message(&message, "Python worker transport failed"),
                canonical_code: Some("pytorch_worker_model_load_failed".to_string()),
            },
        }
        .into_backend_error()
    }

    fn stream_worker_failure_from_message(request_id: &str, message: String) -> BackendError {
        let kind = if message.contains("No model loaded") {
            PyTorchWorkerErrorKind::RuntimeUnavailable
        } else {
            PyTorchWorkerErrorKind::GenerationFailed
        };

        PyTorchWorkerFailure {
            request_id: request_id.to_string(),
            error: PyTorchWorkerError {
                kind,
                message: normalize_worker_error_message(&message, "Python worker transport failed"),
                canonical_code: Some("pytorch_worker_generate_text_stream_failed".to_string()),
            },
        }
        .into_backend_error()
    }

    fn generate_text_worker_failure_from_message(
        request_id: &str,
        message: String,
    ) -> BackendError {
        let kind = if message.contains("No model loaded") {
            PyTorchWorkerErrorKind::RuntimeUnavailable
        } else {
            PyTorchWorkerErrorKind::GenerationFailed
        };

        PyTorchWorkerFailure {
            request_id: request_id.to_string(),
            error: PyTorchWorkerError {
                kind,
                message: normalize_worker_error_message(&message, "Python worker transport failed"),
                canonical_code: Some("pytorch_worker_generate_text_failed".to_string()),
            },
        }
        .into_backend_error()
    }

    fn audio_transcription_worker_failure_from_message(
        request_id: &str,
        message: String,
    ) -> BackendError {
        PyTorchWorkerFailure {
            request_id: request_id.to_string(),
            error: PyTorchWorkerError {
                kind: PyTorchWorkerErrorKind::GenerationFailed,
                message: normalize_worker_error_message(&message, "Python worker transport failed"),
                canonical_code: Some("pytorch_worker_audio_transcription_failed".to_string()),
            },
        }
        .into_backend_error()
    }

    fn audio_transcription_result_from_worker_response(
        request_id: &str,
        response_json: &str,
    ) -> Result<AudioTranscriptionResult, BackendError> {
        let response: PyTorchWorkerResponse<PyTorchAudioTranscriptionResult> =
            serde_json::from_str(response_json).map_err(|error| {
                Self::audio_transcription_worker_failure_from_message(
                    request_id,
                    format!(
                        "Failed to decode PyTorch worker audio_transcription response: {error}"
                    ),
                )
            })?;
        match response {
            PyTorchWorkerResponse::Ok(success) => {
                if success.request_id != request_id {
                    return Err(Self::audio_transcription_worker_failure_from_message(
                        request_id,
                        format!(
                            "PyTorch worker audio_transcription response request_id mismatch: expected {request_id}, got {}",
                            success.request_id
                        ),
                    ));
                }
                Ok(AudioTranscriptionResult {
                    text: success.result.text,
                    language: success.result.language,
                    duration_seconds: success.result.duration_seconds,
                    segments: Vec::new(),
                    metadata: serde_json::Value::Null,
                })
            }
            PyTorchWorkerResponse::Error(failure) => {
                if failure.request_id != request_id {
                    return Err(Self::audio_transcription_worker_failure_from_message(
                        request_id,
                        format!(
                            "PyTorch worker audio_transcription response request_id mismatch: expected {request_id}, got {}",
                            failure.request_id
                        ),
                    ));
                }
                Err(failure.into_backend_error())
            }
        }
    }

    fn unload_worker_failure_from_message(request_id: &str, message: String) -> BackendError {
        PyTorchWorkerFailure {
            request_id: request_id.to_string(),
            error: PyTorchWorkerError {
                kind: PyTorchWorkerErrorKind::GenerationFailed,
                message: normalize_worker_error_message(&message, "Python worker transport failed"),
                canonical_code: Some("pytorch_worker_unload_failed".to_string()),
            },
        }
        .into_backend_error()
    }

    fn init_worker_failure_from_message(request_id: &str, message: String) -> BackendError {
        PyTorchWorkerFailure {
            request_id: request_id.to_string(),
            error: PyTorchWorkerError {
                kind: PyTorchWorkerErrorKind::ModelLoadFailed,
                message: normalize_worker_error_message(&message, "Python worker transport failed"),
                canonical_code: Some("pytorch_worker_init_failed".to_string()),
            },
        }
        .into_backend_error()
    }

    fn generate_text_from_worker_response(
        request_id: &str,
        response_json: &str,
    ) -> Result<String, BackendError> {
        let response: PyTorchWorkerResponse<PyTorchGenerateTextResult> =
            serde_json::from_str(response_json).map_err(|error| {
                Self::generate_text_worker_failure_from_message(
                    request_id,
                    format!("Failed to decode PyTorch worker generate_text response: {error}"),
                )
            })?;
        match response {
            PyTorchWorkerResponse::Ok(success) => {
                if success.request_id != request_id {
                    return Err(Self::generate_text_worker_failure_from_message(
                        request_id,
                        format!(
                            "PyTorch worker generate_text response request_id mismatch: expected {request_id}, got {}",
                            success.request_id
                        ),
                    ));
                }
                Ok(success.result.text)
            }
            PyTorchWorkerResponse::Error(failure) => {
                if failure.request_id != request_id {
                    return Err(Self::generate_text_worker_failure_from_message(
                        request_id,
                        format!(
                            "PyTorch worker generate_text response request_id mismatch: expected {request_id}, got {}",
                            failure.request_id
                        ),
                    ));
                }
                Err(failure.into_backend_error())
            }
        }
    }

    fn validate_transformers_load_envelope(
        envelope: &PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest>,
    ) -> Result<(), BackendError> {
        if envelope.contract_version != PYTORCH_WORKER_CONTRACT_VERSION {
            return Err(BackendError::Config(format!(
                "Unsupported PyTorch worker load envelope contract version {}",
                envelope.contract_version
            )));
        }
        if envelope.operation != PyTorchWorkerOperation::LoadTransformersModel {
            return Err(BackendError::Config(format!(
                "Unexpected PyTorch worker operation {:?} for Transformers load",
                envelope.operation
            )));
        }
        if let Some(model_source) = &envelope.payload.model_source {
            if let Err(diagnostics) = model_source.validate_for_backend_load() {
                let codes = diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(BackendError::Config(format!(
                    "Invalid PyTorch worker resolved model source: {codes}"
                )));
            }
        }
        Ok(())
    }

    fn validate_generate_text_envelope(
        envelope: &PyTorchWorkerEnvelope<PyTorchGenerateTextRequest>,
    ) -> Result<(), BackendError> {
        Self::validate_generate_text_envelope_operation(
            envelope,
            PyTorchWorkerOperation::GenerateText,
        )
    }

    fn validate_generate_text_stream_envelope(
        envelope: &PyTorchWorkerEnvelope<PyTorchGenerateTextRequest>,
    ) -> Result<(), BackendError> {
        Self::validate_generate_text_envelope_operation(
            envelope,
            PyTorchWorkerOperation::GenerateTextStream,
        )
    }

    fn validate_generate_text_envelope_operation(
        envelope: &PyTorchWorkerEnvelope<PyTorchGenerateTextRequest>,
        expected_operation: PyTorchWorkerOperation,
    ) -> Result<(), BackendError> {
        if envelope.contract_version != PYTORCH_WORKER_CONTRACT_VERSION {
            return Err(BackendError::Config(format!(
                "Unsupported PyTorch worker generate_text envelope contract version {}",
                envelope.contract_version
            )));
        }
        if envelope.operation != expected_operation {
            return Err(BackendError::Config(format!(
                "Unexpected PyTorch worker operation {:?} for text generation",
                envelope.operation
            )));
        }
        if envelope.payload.prompt.trim().is_empty() {
            return Err(BackendError::Config(
                "PyTorch worker generate_text envelope requires a prompt".to_string(),
            ));
        }
        if let Some(unsupported_key) = envelope
            .payload
            .transformers_kwargs
            .keys()
            .find(|key| !ALLOWED_TRANSFORMERS_GENERATE_KWARGS.contains(&key.as_str()))
        {
            return Err(BackendError::Config(format!(
                "PyTorch worker generate_text envelope contains unsupported transformers_kwargs key '{unsupported_key}'"
            )));
        }
        Ok(())
    }

    fn validate_audio_transcription_envelope(
        envelope: &PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest>,
    ) -> Result<(), BackendError> {
        if envelope.contract_version != PYTORCH_WORKER_CONTRACT_VERSION {
            return Err(BackendError::Config(format!(
                "Unsupported PyTorch worker audio_transcription envelope contract version {}",
                envelope.contract_version
            )));
        }
        if envelope.operation != PyTorchWorkerOperation::TranscribeAudio {
            return Err(BackendError::Config(format!(
                "Unexpected PyTorch worker operation {:?} for audio transcription",
                envelope.operation
            )));
        }
        if envelope.payload.model_path.trim().is_empty() {
            return Err(BackendError::Config(
                "PyTorch worker audio_transcription envelope requires a model_path".to_string(),
            ));
        }
        if envelope.payload.audio_base64.trim().is_empty() {
            return Err(BackendError::Config(
                "PyTorch worker audio_transcription envelope requires audio_base64".to_string(),
            ));
        }
        if !envelope.payload.extra_options.is_null() {
            return Err(BackendError::Config(
                "PyTorch worker audio_transcription envelope does not support extra_options yet"
                    .to_string(),
            ));
        }
        Ok(())
    }

    fn audio_transcription_envelope_from_request(
        request_id: impl Into<String>,
        request: AudioTranscriptionRequest,
    ) -> Result<PyTorchWorkerEnvelope<PyTorchAudioTranscriptionRequest>, BackendError> {
        let audio_base64 = Self::audio_base64_from_request(&request)?;
        let model_path = request.model;
        if model_path.trim().is_empty() {
            return Err(BackendError::Config(
                "PyTorch audio transcription requires a model".to_string(),
            ));
        }

        let envelope = PyTorchWorkerEnvelope::new(
            request_id,
            PyTorchWorkerOperation::TranscribeAudio,
            PyTorchAudioTranscriptionRequest {
                model_path,
                audio_base64,
                device: "auto".to_string(),
                language: request
                    .language
                    .and_then(|value| (!value.trim().is_empty()).then_some(value)),
                prompt: request
                    .prompt
                    .and_then(|value| (!value.trim().is_empty()).then_some(value)),
                task: request
                    .task
                    .and_then(|value| (!value.trim().is_empty()).then_some(value)),
                chunk_length_s: request.chunk_length_s,
                extra_options: request.extra_options,
            },
        );
        Self::validate_audio_transcription_envelope(&envelope)?;
        Ok(envelope)
    }

    fn generate_text_request(
        prompt: String,
        system_prompt: Option<String>,
        max_tokens: i64,
        temperature: f64,
        top_p: f64,
        top_k: Option<u32>,
        masked_prompt_json: Option<String>,
    ) -> PyTorchGenerateTextRequest {
        PyTorchGenerateTextRequest {
            prompt,
            system_prompt,
            max_tokens,
            temperature,
            top_p,
            masked_prompt_json,
            denoising_steps: None,
            block_length: None,
            transformers_kwargs: Self::generate_text_transformers_kwargs(top_k),
        }
    }

    fn generate_text_envelope(
        request_id: impl Into<String>,
        operation: PyTorchWorkerOperation,
        prompt: String,
        system_prompt: Option<String>,
        max_tokens: i64,
        temperature: f64,
        top_p: f64,
        top_k: Option<u32>,
        masked_prompt_json: Option<String>,
    ) -> PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> {
        PyTorchWorkerEnvelope::new(
            request_id,
            operation,
            Self::generate_text_request(
                prompt,
                system_prompt,
                max_tokens,
                temperature,
                top_p,
                top_k,
                masked_prompt_json,
            ),
        )
    }

    fn generate_text_transformers_kwargs(top_k: Option<u32>) -> BTreeMap<String, Value> {
        let mut kwargs = BTreeMap::new();
        if let Some(top_k) = top_k {
            kwargs.insert("top_k".to_string(), serde_json::json!(top_k));
        }
        kwargs
    }

    #[cfg(test)]
    fn transformers_load_args_from_request(
        request: &PyTorchTransformersLoadRequest,
    ) -> PyTorchTransformersLoadArgs {
        PyTorchTransformersLoadArgs {
            model_path: request.entry_path.clone(),
            device: request.device.clone().unwrap_or_else(|| "auto".to_string()),
            model_type: request.model_type_hint.clone(),
            trust_policy: request.trust_policy.clone(),
        }
    }

    #[allow(dead_code)]
    fn transformers_task_profile_from_evidence(
        evidence: &TaskEvidence,
    ) -> Result<PyTorchTransformersTaskProfile, BackendError> {
        let entry = resolve_task_registry_entry_from_evidence(evidence).map_err(|diagnostic| {
            BackendError::Config(format!(
                "PyTorch/Transformers task evidence did not resolve: {:?}: {}",
                diagnostic.kind, diagnostic.message
            ))
        })?;
        Self::transformers_task_profile_from_registry_entry(&entry)
    }

    fn transformers_task_profile_from_registry_entry(
        entry: &TaskRegistryEntry,
    ) -> Result<PyTorchTransformersTaskProfile, BackendError> {
        match entry.task_id {
            InferenceTaskId::TextGeneration | InferenceTaskId::ChatCompletion => {
                Ok(PyTorchTransformersTaskProfile {
                    task_id: entry.task_id.clone(),
                    canonical_task_label: entry.canonical_label().to_string(),
                    loader: PyTorchTransformersModelLoader::CausalLm,
                    required_components: entry.required_components.clone(),
                })
            }
            InferenceTaskId::AudioTranscription => Ok(PyTorchTransformersTaskProfile {
                task_id: entry.task_id.clone(),
                canonical_task_label: entry.canonical_label().to_string(),
                loader: PyTorchTransformersModelLoader::AutomaticSpeechRecognition,
                required_components: entry.required_components.clone(),
            }),
            ref task_id => Err(BackendError::Config(format!(
                "PyTorch/Transformers load does not support canonical task {} yet",
                task_id.canonical_label()
            ))),
        }
    }

    #[allow(dead_code)]
    fn transformers_generation_option_mapping(
        options: &GenerationOptions,
    ) -> PyTorchGenerationOptionMapping {
        let mut kwargs = Map::new();
        let mut diagnostics = Vec::new();

        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "length.max_new_tokens",
            "max_new_tokens",
            options.length.max_new_tokens,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "length.min_new_tokens",
            "min_new_tokens",
            options.length.min_new_tokens,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "length.max_length",
            "max_length",
            options.length.max_length,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "sampling.temperature",
            "temperature",
            options.sampling.temperature,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "sampling.top_p",
            "top_p",
            options.sampling.top_p,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "sampling.top_k",
            "top_k",
            options.sampling.top_k,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "sampling.repetition_penalty",
            "repetition_penalty",
            options.sampling.repetition_penalty,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "search.num_beams",
            "num_beams",
            options.search.num_beams,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "search.num_return_sequences",
            "num_return_sequences",
            options.search.num_return_sequences,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "cache.use_cache",
            "use_cache",
            options.cache.use_cache,
            OptionSupportState::Honored,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "special_tokens.bos_token_id",
            "bos_token_id",
            options.special_tokens.bos_token_id,
            OptionSupportState::Mapped,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "special_tokens.eos_token_id",
            "eos_token_id",
            options.special_tokens.eos_token_id,
            OptionSupportState::Mapped,
        );
        Self::map_generation_option(
            &mut kwargs,
            &mut diagnostics,
            "special_tokens.pad_token_id",
            "pad_token_id",
            options.special_tokens.pad_token_id,
            OptionSupportState::Mapped,
        );

        if let Some(seed) = options.sampling.seed {
            diagnostics.push(Self::generation_option_diagnostic(
                "sampling.seed",
                OptionSupportState::Unsupported,
                Some(format!(
                    "PyTorch/Transformers seed handling is not wired into the worker yet ({seed})"
                )),
            ));
        }
        if !options.stopping.stop_strings.is_empty() {
            diagnostics.push(Self::generation_option_diagnostic(
                "stopping.stop_strings",
                OptionSupportState::Unsupported,
                Some("stop string criteria are not wired into the PyTorch worker yet".to_string()),
            ));
        }
        if !options.stopping.eos_token_ids.is_empty() {
            kwargs.insert(
                "eos_token_id".to_string(),
                serde_json::json!(options.stopping.eos_token_ids),
            );
            diagnostics.push(Self::generation_option_diagnostic(
                "stopping.eos_token_ids",
                OptionSupportState::Mapped,
                Some("mapped to Transformers eos_token_id".to_string()),
            ));
        }
        if options.cache.kv_cache_checkpoint_requested == Some(true) {
            diagnostics.push(Self::generation_option_diagnostic(
                "cache.kv_cache_checkpoint_requested",
                OptionSupportState::Mapped,
                Some(
                    "handled by Pantograph KV-cache publication outside GenerationConfig"
                        .to_string(),
                ),
            ));
        }
        if options.output.return_logprobs == Some(true) {
            diagnostics.push(Self::generation_option_diagnostic(
                "output.return_logprobs",
                OptionSupportState::Unsupported,
                Some("logprob output is not exposed by the PyTorch worker yet".to_string()),
            ));
        }
        if options.output.return_token_ids == Some(true) {
            diagnostics.push(Self::generation_option_diagnostic(
                "output.return_token_ids",
                OptionSupportState::Unsupported,
                Some("token-id output is not exposed by the PyTorch worker yet".to_string()),
            ));
        }
        let invalid_backend_extension_paths = options
            .backend_extension_scope_diagnostics()
            .into_iter()
            .map(|diagnostic| {
                let path = diagnostic.option_path.clone();
                diagnostics.push(diagnostic);
                path
            })
            .collect::<Vec<_>>();
        for (key, value) in &options.backend_extensions {
            let option_path = format!("backend_extensions.{key}");
            if invalid_backend_extension_paths.contains(&option_path) {
                continue;
            }
            if let Some(transformers_key) = key.strip_prefix("transformers:") {
                kwargs.insert(transformers_key.to_string(), value.clone());
                diagnostics.push(Self::generation_option_diagnostic(
                    option_path,
                    OptionSupportState::Mapped,
                    Some(format!(
                        "mapped to Transformers extension key {transformers_key}"
                    )),
                ));
            } else {
                diagnostics.push(Self::generation_option_diagnostic(
                    option_path,
                    OptionSupportState::Unsupported,
                    Some("backend extension is not scoped to Transformers".to_string()),
                ));
            }
        }

        PyTorchGenerationOptionMapping {
            kwargs,
            diagnostics,
        }
    }

    fn map_generation_option<T: serde::Serialize>(
        kwargs: &mut Map<String, Value>,
        diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
        option_path: &'static str,
        transformers_key: &'static str,
        value: Option<T>,
        state: OptionSupportState,
    ) {
        if let Some(value) = value {
            kwargs.insert(transformers_key.to_string(), serde_json::json!(value));
            diagnostics.push(Self::generation_option_diagnostic(
                option_path,
                state,
                Some(format!("mapped to Transformers {transformers_key}")),
            ));
        }
    }

    fn generation_option_diagnostic(
        option_path: impl Into<String>,
        state: OptionSupportState,
        message: Option<String>,
    ) -> OptionCompatibilityDiagnostic {
        OptionCompatibilityDiagnostic {
            option_path: option_path.into(),
            state,
            backend_key: Some("pytorch".to_string()),
            message,
        }
    }

    fn audio_base64_from_request(
        request: &AudioTranscriptionRequest,
    ) -> Result<String, BackendError> {
        if let Some(audio) = &request.audio {
            let data_base64 = audio.data_base64.trim();
            if !data_base64.is_empty() {
                return Ok(data_base64.to_string());
            }
        }

        if request
            .audio_ref
            .as_deref()
            .is_some_and(|audio_ref| !audio_ref.trim().is_empty())
        {
            return Err(BackendError::Config(
                "PyTorch audio transcription requires encoded audio; audio_ref resolution is owned by the host adapter".to_string(),
            ));
        }

        Err(BackendError::Config(
            "PyTorch audio transcription requires encoded audio".to_string(),
        ))
    }

    async fn load_model_with_trust_policy(
        &mut self,
        model_path: &str,
        device: &str,
        model_type: Option<&str>,
        trust_policy: PyTorchTransformersTrustPolicy,
    ) -> Result<LoadedModelInfo, BackendError> {
        let envelope = Self::transformers_load_envelope_from_direct_path(
            format!("pytorch-direct-load-{}", Uuid::new_v4().simple()),
            model_path,
            Some(device),
            model_type,
            trust_policy,
        );
        self.load_transformers_envelope(envelope).await
    }

    /// Unload the current model and free GPU memory.
    pub async fn unload_model(&mut self) -> Result<(), BackendError> {
        let request_id = format!("pytorch-unload-{}", Uuid::new_v4().simple());
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<(), BackendError> {
                let worker = pytorch_worker::worker_module(py).map_err(|e| {
                    Self::unload_worker_failure_from_message(
                        &request_id,
                        format!("Failed to get worker module: {}", e),
                    )
                })?;
                worker.call_method0("unload_model").map_err(|e| {
                    Self::unload_worker_failure_from_message(
                        &request_id,
                        format!("Unload failed: {}", e),
                    )
                })?;
                Ok(())
            })
        })
        .await
        .map_err(|e| BackendError::Inference(task_join_error_message(e)))??;

        self.loaded_model = None;
        Ok(())
    }

    /// Generate a complete response (non-streaming).
    ///
    /// When `masked_prompt_json` is `Some`, the JSON is passed through to the
    /// Python worker so it can perform masked (anchor-preserving) generation.
    pub async fn generate(
        &self,
        prompt: String,
        system_prompt: Option<String>,
        max_tokens: i64,
        temperature: f64,
        top_p: f64,
        masked_prompt_json: Option<String>,
    ) -> Result<String, BackendError> {
        self.generate_with_top_k(
            prompt,
            system_prompt,
            max_tokens,
            temperature,
            top_p,
            None,
            masked_prompt_json,
        )
        .await
    }

    async fn generate_with_top_k(
        &self,
        prompt: String,
        system_prompt: Option<String>,
        max_tokens: i64,
        temperature: f64,
        top_p: f64,
        top_k: Option<u32>,
        masked_prompt_json: Option<String>,
    ) -> Result<String, BackendError> {
        let request_id = format!("pytorch-generate-text-{}", Uuid::new_v4().simple());
        let envelope = Self::generate_text_envelope(
            request_id.clone(),
            PyTorchWorkerOperation::GenerateText,
            prompt,
            system_prompt,
            max_tokens,
            temperature,
            top_p,
            top_k,
            masked_prompt_json,
        );
        Self::validate_generate_text_envelope(&envelope)?;
        let envelope_json = serde_json::to_string(&envelope).map_err(|error| {
            BackendError::Config(format!(
                "Failed to encode PyTorch worker generate_text envelope: {error}"
            ))
        })?;

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<String, BackendError> {
                let worker = pytorch_worker::worker_module(py).map_err(|e| {
                    Self::generate_text_worker_failure_from_message(
                        &request_id,
                        format!("Failed to get worker module: {}", e),
                    )
                })?;

                let result = worker
                    .call_method1("generate_text_from_envelope", (envelope_json,))
                    .map_err(|e| {
                        Self::generate_text_worker_failure_from_message(
                            &request_id,
                            format!("PyTorch worker generate_text envelope failed: {}", e),
                        )
                    })?;

                let response_json = result.extract::<String>().map_err(|e| {
                    Self::generate_text_worker_failure_from_message(
                        &request_id,
                        format!(
                            "Failed to extract PyTorch worker generate_text response: {}",
                            e
                        ),
                    )
                })?;
                Self::generate_text_from_worker_response(&request_id, &response_json)
            })
        })
        .await
        .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
    }

    /// Generate tokens as a stream via an mpsc channel.
    ///
    /// Spawns a blocking task that iterates the Python generator and sends
    /// each token through the channel. When `masked_prompt_json` is `Some`,
    /// it is forwarded to the Python worker for masked generation.
    pub fn generate_stream(
        &self,
        prompt: String,
        system_prompt: Option<String>,
        max_tokens: i64,
        temperature: f64,
        top_p: f64,
        masked_prompt_json: Option<String>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>> {
        self.generate_stream_with_top_k(
            prompt,
            system_prompt,
            max_tokens,
            temperature,
            top_p,
            None,
            masked_prompt_json,
        )
    }

    fn generate_stream_with_top_k(
        &self,
        prompt: String,
        system_prompt: Option<String>,
        max_tokens: i64,
        temperature: f64,
        top_p: f64,
        top_k: Option<u32>,
        masked_prompt_json: Option<String>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>> {
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ChatChunk, BackendError>>(32);
        let request_id = format!("pytorch-generate-text-stream-{}", Uuid::new_v4().simple());
        let envelope = Self::generate_text_envelope(
            request_id.clone(),
            PyTorchWorkerOperation::GenerateTextStream,
            prompt,
            system_prompt,
            max_tokens,
            temperature,
            top_p,
            top_k,
            masked_prompt_json,
        );

        if let Err(error) = Self::validate_generate_text_stream_envelope(&envelope) {
            let _ = tx.try_send(Err(error));
            return Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));
        }

        let envelope_json = match serde_json::to_string(&envelope) {
            Ok(envelope_json) => envelope_json,
            Err(error) => {
                let _ = tx.try_send(Err(BackendError::Config(format!(
                    "Failed to encode PyTorch worker generate_text_stream envelope: {error}"
                ))));
                return Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx));
            }
        };

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let worker = match pytorch_worker::worker_module(py) {
                    Ok(w) => w,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(Self::stream_worker_failure_from_message(
                            &request_id,
                            format!("Failed to get worker module: {}", e),
                        )));
                        return;
                    }
                };

                let setup_response_json = match worker
                    .call_method1(
                        "generate_text_stream_setup_from_envelope",
                        (&envelope_json,),
                    )
                    .and_then(|result| result.extract::<String>())
                {
                    Ok(response_json) => response_json,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(Self::stream_worker_failure_from_message(
                            &request_id,
                            format!("PyTorch worker generate_text_stream setup failed: {}", e),
                        )));
                        return;
                    }
                };
                if let Err(error) =
                    Self::stream_setup_from_worker_response(&request_id, &setup_response_json)
                {
                    let _ = tx.blocking_send(Err(error));
                    return;
                }

                let generator = match worker
                    .call_method1("generate_text_stream_from_envelope", (envelope_json,))
                {
                    Ok(g) => g,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(Self::stream_worker_failure_from_message(
                            &request_id,
                            format!("PyTorch worker generate_text_stream envelope failed: {}", e),
                        )));
                        return;
                    }
                };

                // Iterate the Python generator
                let iter = match generator.try_iter() {
                    Ok(it) => it,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(Self::stream_worker_failure_from_message(
                            &request_id,
                            format!("Generator is not iterable: {}", e),
                        )));
                        return;
                    }
                };

                for item in iter {
                    match item {
                        Ok(token_obj) => match token_obj.extract::<String>() {
                            Ok(token) => {
                                if tx
                                    .blocking_send(Ok(ChatChunk {
                                        content: Some(token),
                                        done: false,
                                        usage: None,
                                        cache_handle_id: None,
                                    }))
                                    .is_err()
                                {
                                    return;
                                }
                            }
                            Err(e) => {
                                let _ = tx.blocking_send(Err(
                                    Self::stream_worker_failure_from_message(
                                        &request_id,
                                        format!("Token extraction failed: {}", e),
                                    ),
                                ));
                                return;
                            }
                        },
                        Err(e) => {
                            let _ =
                                tx.blocking_send(Err(Self::stream_worker_failure_from_message(
                                    &request_id,
                                    format!("Generator error: {}", e),
                                )));
                            return;
                        }
                    }
                }

                // Signal completion
                let _ = tx.blocking_send(Ok(ChatChunk {
                    content: None,
                    done: true,
                    usage: None,
                    cache_handle_id: None,
                }));
            });
        });

        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }
}

impl Default for PyTorchBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for PyTorchBackend {
    fn name(&self) -> &'static str {
        "PyTorch"
    }

    fn description(&self) -> &'static str {
        "In-process PyTorch inference for dLLM, Sherry, and HuggingFace models"
    }

    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    async fn start(
        &mut self,
        config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        let was_ready = self.ready;

        // Initialise the Python worker module
        let request_id = format!("pytorch-worker-init-{}", Uuid::new_v4().simple());
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                pytorch_worker::ensure_worker_initialised(py).map_err(|e| {
                    Self::init_worker_failure_from_message(
                        &request_id,
                        format!("Failed to initialise Python worker: {}", e),
                    )
                })
            })
        })
        .await
        .map_err(|e| BackendError::StartupFailed(task_join_error_message(e)))??;

        // Log the transformers version for diagnostics
        let tf_version = tokio::task::spawn_blocking(|| {
            Python::with_gil(|py| -> String {
                py.import("transformers")
                    .and_then(|m| m.getattr("__version__"))
                    .and_then(|v| v.extract::<String>())
                    .unwrap_or_else(|_| "unknown".into())
            })
        })
        .await
        .unwrap_or_else(|_| "unknown".into());
        log::info!("PyTorch backend: transformers {}", tf_version);

        // If config includes a model_path, load it immediately
        if let Some(ref model_path) = config.model_path {
            let device = config.device.as_deref().unwrap_or("auto");
            let model_type = config.model_type.as_deref();
            let model_path = model_path.to_string_lossy().to_string();

            if self.can_reuse_loaded_model(&model_path, device, model_type) {
                self.ready = true;
                log::info!("PyTorch backend: reusing loaded model {}", model_path);
                return Ok(BackendStartOutcome {
                    runtime_reused: Some(true),
                    lifecycle_decision_reason: Some("runtime_reused".to_string()),
                });
            }

            self.load_model(&model_path, device, model_type).await?;

            return Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            });
        }

        self.ready = true;
        Ok(BackendStartOutcome {
            runtime_reused: Some(was_ready),
            lifecycle_decision_reason: Some(
                if was_ready {
                    "runtime_reused"
                } else {
                    "runtime_ready"
                }
                .to_string(),
            ),
        })
    }

    fn stop(&mut self) {
        // Best-effort unload — can't await in a sync fn, so use blocking
        let had_model = self.loaded_model.is_some();
        self.loaded_model = None;
        self.ready = false;

        if had_model {
            std::thread::spawn(|| {
                Python::with_gil(|py| {
                    if let Ok(worker) = pytorch_worker::worker_module(py) {
                        let _ = worker.call_method0("unload_model");
                    }
                });
            });
        }
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn health_check(&self) -> bool {
        if !self.ready {
            return false;
        }
        tokio::task::spawn_blocking(|| {
            Python::with_gil(|py| pytorch_worker::worker_module(py).is_ok())
        })
        .await
        .unwrap_or(false)
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        if !self.ready {
            return Err(BackendError::NotReady);
        }

        let request: serde_json::Value = serde_json::from_str(&request_json)
            .map_err(|e| BackendError::Inference(format!("Invalid request JSON: {}", e)))?;

        let prompt = extract_prompt_from_messages(&request)?;
        let system_prompt = extract_system_prompt(&request);
        let max_tokens = request
            .get("max_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(512);
        let temperature = request
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);
        let top_p = request.get("top_p").and_then(|v| v.as_f64()).unwrap_or(1.0);
        let top_k = request
            .get("top_k")
            .and_then(|value| value.as_u64())
            .and_then(|value| u32::try_from(value).ok());

        Ok(self.generate_stream_with_top_k(
            prompt,
            system_prompt,
            max_tokens,
            temperature,
            top_p,
            top_k,
            None,
        ))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Err(BackendError::Inference(
            "Embeddings not supported by PyTorch backend".to_string(),
        ))
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Err(BackendError::Inference(
            "Reranking not supported by PyTorch backend".to_string(),
        ))
    }

    async fn transcribe_audio(
        &self,
        request: AudioTranscriptionRequest,
    ) -> Result<AudioTranscriptionResult, BackendError> {
        if !self.ready {
            return Err(BackendError::NotReady);
        }

        let request_id = format!("pytorch-audio-transcription-{}", Uuid::new_v4().simple());
        let envelope =
            Self::audio_transcription_envelope_from_request(request_id.clone(), request)?;
        let envelope_json = serde_json::to_string(&envelope).map_err(|error| {
            BackendError::Config(format!(
                "Failed to encode PyTorch worker audio_transcription envelope: {error}"
            ))
        })?;

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<AudioTranscriptionResult, BackendError> {
                let worker = pytorch_worker::worker_module(py).map_err(|e| {
                    Self::audio_transcription_worker_failure_from_message(
                        &request_id,
                        format!("Failed to get worker module: {}", e),
                    )
                })?;

                let response_json = worker
                    .call_method1("transcribe_audio_from_envelope", (envelope_json,))
                    .map_err(|e| {
                        Self::audio_transcription_worker_failure_from_message(
                            &request_id,
                            format!("PyTorch worker audio_transcription envelope failed: {e}"),
                        )
                    })?
                    .extract::<String>()
                    .map_err(|e| {
                        Self::audio_transcription_worker_failure_from_message(
                            &request_id,
                            format!(
                                "PyTorch worker audio_transcription response was not JSON text: {e}"
                            ),
                        )
                    })?;
                Self::audio_transcription_result_from_worker_response(&request_id, &response_json)
            })
        })
        .await
        .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
    }

    async fn kv_cache_runtime_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<KvCacheRuntimeFingerprint, BackendError> {
        Ok(Self::kv_cache_runtime_fingerprint_for_loaded_model(
            self.active_loaded_model()?,
        ))
    }

    async fn kv_cache_model_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<ModelFingerprint, BackendError> {
        Ok(Self::kv_cache_model_fingerprint_for_loaded_model(
            self.active_loaded_model()?,
        ))
    }

    async fn save_kv_cache_slot(&self, slot_id: u32, path: &Path) -> Result<(), BackendError> {
        Self::require_live_kv_slot(slot_id)?;
        let path = path.to_path_buf();
        let request_id = format!("pytorch-kv-slot-save-{}", Uuid::new_v4().simple());
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<(), BackendError> {
                let worker = pytorch_worker::worker_module(py).map_err(|e| {
                    kv_worker_failure_from_message(
                        &request_id,
                        "pytorch_worker_kv_save_failed",
                        format!("Failed to get worker module: {}", e),
                    )
                })?;
                worker
                    .call_method1("save_live_kv_cache", (path.to_string_lossy().to_string(),))
                    .map_err(|e| {
                        kv_worker_failure_from_message(
                            &request_id,
                            "pytorch_worker_kv_save_failed",
                            format!("PyTorch KV save failed: {}", e),
                        )
                    })?;
                Ok(())
            })
        })
        .await
        .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
    }

    async fn restore_kv_cache_slot(&self, slot_id: u32, path: &Path) -> Result<(), BackendError> {
        Self::require_live_kv_slot(slot_id)?;
        let path = path.to_path_buf();
        let request_id = format!("pytorch-kv-slot-restore-{}", Uuid::new_v4().simple());
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<(), BackendError> {
                let worker = pytorch_worker::worker_module(py).map_err(|e| {
                    kv_worker_failure_from_message(
                        &request_id,
                        "pytorch_worker_kv_restore_failed",
                        format!("Failed to get worker module: {}", e),
                    )
                })?;
                worker
                    .call_method1(
                        "restore_live_kv_cache",
                        (path.to_string_lossy().to_string(),),
                    )
                    .map_err(|e| {
                        kv_worker_failure_from_message(
                            &request_id,
                            "pytorch_worker_kv_restore_failed",
                            format!("PyTorch KV restore failed: {}", e),
                        )
                    })?;
                Ok(())
            })
        })
        .await
        .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
    }

    async fn clear_kv_cache_slot(&self, slot_id: u32) -> Result<(), BackendError> {
        Self::require_live_kv_slot(slot_id)?;
        let request_id = format!("pytorch-kv-slot-clear-{}", Uuid::new_v4().simple());
        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<(), BackendError> {
                let worker = pytorch_worker::worker_module(py).map_err(|e| {
                    kv_worker_failure_from_message(
                        &request_id,
                        "pytorch_worker_kv_clear_failed",
                        format!("Failed to get worker module: {}", e),
                    )
                })?;
                worker.call_method0("clear_live_kv_cache").map_err(|e| {
                    kv_worker_failure_from_message(
                        &request_id,
                        "pytorch_worker_kv_clear_failed",
                        format!("PyTorch KV clear failed: {}", e),
                    )
                })?;
                Ok(())
            })
        })
        .await
        .map_err(|e| BackendError::Inference(task_join_error_message(e)))?
    }

    async fn truncate_kv_cache_data(
        &self,
        data: &[u8],
        token_position: usize,
        _active_config: Option<&BackendConfig>,
    ) -> Result<Vec<u8>, BackendError> {
        let temp_path = std::env::temp_dir().join(format!(
            "pantograph-pytorch-kv-truncate-{}.bin",
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&temp_path, data)
            .map_err(|e| BackendError::Inference(format!("Failed to write KV temp file: {}", e)))?;
        let truncate_result = tokio::task::spawn_blocking({
            let temp_path = temp_path.clone();
            let request_id = format!("pytorch-kv-truncate-{}", Uuid::new_v4().simple());
            move || {
                Python::with_gil(|py| -> Result<(), BackendError> {
                    let worker = pytorch_worker::worker_module(py).map_err(|e| {
                        kv_worker_failure_from_message(
                            &request_id,
                            "pytorch_worker_kv_truncate_failed",
                            format!("Failed to get worker module: {}", e),
                        )
                    })?;
                    worker
                        .call_method1(
                            "truncate_kv_cache_file",
                            (temp_path.to_string_lossy().to_string(), token_position),
                        )
                        .map_err(|e| {
                            kv_worker_failure_from_message(
                                &request_id,
                                "pytorch_worker_kv_truncate_failed",
                                format!("PyTorch KV truncate failed: {}", e),
                            )
                        })?;
                    Ok(())
                })
            }
        })
        .await
        .map_err(|e| BackendError::Inference(task_join_error_message(e)))?;
        let read_result = std::fs::read(&temp_path)
            .map_err(|e| BackendError::Inference(format!("Failed to read KV temp file: {}", e)));
        let _ = std::fs::remove_file(&temp_path);
        truncate_result?;
        read_result
    }
}

/// Extract the last user message from OpenAI-format messages array.
fn extract_prompt_from_messages(request: &serde_json::Value) -> Result<String, BackendError> {
    let messages = request
        .get("messages")
        .and_then(|m| m.as_array())
        .ok_or_else(|| BackendError::Inference("Missing 'messages' array".to_string()))?;

    messages
        .iter()
        .rev()
        .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .and_then(|m| m.get("content").and_then(|c| c.as_str()))
        .map(|s| s.to_string())
        .ok_or_else(|| BackendError::Inference("No user message found".to_string()))
}

/// Extract the system prompt from OpenAI-format messages array, if present.
fn extract_system_prompt(request: &serde_json::Value) -> Option<String> {
    request
        .get("messages")
        .and_then(|m| m.as_array())
        .and_then(|msgs| {
            msgs.iter()
                .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
                .and_then(|m| m.get("content").and_then(|c| c.as_str()))
                .map(|s| s.to_string())
        })
}

#[cfg(test)]
#[path = "pytorch_tests.rs"]
mod tests;
