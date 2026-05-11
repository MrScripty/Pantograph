//! Candle backend implementation
//!
//! This backend provides in-process inference using Hugging Face Candle.
//! It supports CUDA acceleration and various model architectures.

use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;

use super::{
    openai_embedding_token_count_for_single_result, unavailable_runtime_variant_capability,
    BackendCapabilities, BackendCapabilityFacts, BackendComponentCapability, BackendConfig,
    BackendError, BackendFeatureCapabilityFacts, BackendFeatureSupport,
    BackendModelSourceCapabilityFacts, BackendStartOutcome, BackendTaskCapability, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use crate::device_contracts::{DeviceResolutionDiagnosticCode, InferenceDeviceClass};
use crate::model_contracts::{
    resolve_task_registry_entry_from_evidence, InferenceModality, InferenceTaskId,
    ModelValidationState, PackageFactStatus, ProcessorComponentKind, ResolvedModelPackageFacts,
    ResolvedModelSource,
};
use crate::process::ProcessSpawner;
use crate::types::{RerankRequest, RerankResponse};
use crate::{BackendHintLabel, ModelArtifactKind};

/// Candle backend for in-process inference
///
/// This backend runs inference directly in the process using Candle.
/// It supports embedding models with CUDA acceleration.
pub struct CandleBackend {
    /// HTTP client for API requests (to local Axum server)
    http_client: reqwest::Client,
    /// Base URL of the local server
    base_url: Option<String>,
    /// Whether the backend is ready
    ready: bool,
}

/// Narrow staged load plan for Candle embedding models.
///
/// This is a backend-local fact projection. It resolves what Candle would load
/// from package facts without choosing runtime residency or scheduler policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandleEmbeddingLoadPlan {
    pub source: ResolvedModelSource,
    pub model_dir: PathBuf,
    pub config_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub safetensors_path: PathBuf,
    pub dtype: CandleLoadDType,
    pub device: CandleLoadDevice,
    pub model_type: String,
    pub architecture: Option<String>,
}

/// Dtype accepted by the staged Candle embedding loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleLoadDType {
    F32,
    F16,
    BF16,
}

/// Device hint accepted by the staged Candle embedding loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleLoadDevice {
    Auto,
    Cpu,
    Cuda { index: usize },
}

/// Candle resources loaded from a staged embedding load plan.
///
/// This is still backend-local staging: it validates concrete Candle inputs and
/// loads weight tensors, but it does not construct an executable model module,
/// start a runtime, or make scheduling/residency decisions.
#[cfg(feature = "backend-candle")]
#[derive(Debug)]
pub struct CandleEmbeddingLoadResources {
    pub device: candle_core::Device,
    pub dtype: candle_core::DType,
    pub tokenizer: tokenizers::Tokenizer,
    pub tensors: std::collections::HashMap<String, candle_core::Tensor>,
}

impl CandleBackend {
    /// Create a new Candle backend
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url: None,
            ready: false,
        }
    }

    /// Get static capabilities (for registry info before instantiation)
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            vision: false, // Candle doesn't support vision models yet
            image_generation: false,
            embeddings: true, // Primary use case
            reranking: false,
            gpu: true,               // CUDA support
            device_selection: false, // Limited device selection
            streaming: false,        // Not supported yet
            tool_calling: false,     // Not supported
            external_connection: false,
            facts: BackendCapabilityFacts {
                tasks: vec![BackendTaskCapability::stable(
                    InferenceTaskId::Embedding,
                    vec![InferenceModality::Text],
                    vec![InferenceModality::Embedding],
                )],
                preprocessing: BackendComponentCapability::RequiresPackageComponent,
                postprocessing: BackendComponentCapability::NotRequired,
                model_sources: BackendModelSourceCapabilityFacts {
                    artifact_kinds: vec![ModelArtifactKind::HfCompatibleDirectory],
                    backend_hints: vec![BackendHintLabel::Candle],
                    custom_code: BackendFeatureSupport::Unsupported,
                },
                features: BackendFeatureCapabilityFacts {
                    streaming: BackendFeatureSupport::Unsupported,
                    device_selection: BackendFeatureSupport::Unsupported,
                    external_connection: BackendFeatureSupport::Unsupported,
                    kv_cache: BackendFeatureSupport::Unsupported,
                },
                runtime_variants: vec![
                    unavailable_runtime_variant_capability(
                        "candle",
                        "candle.cpu",
                        InferenceDeviceClass::Cpu,
                        DeviceResolutionDiagnosticCode::CandidateUnavailable,
                        "Candle executable model loading is not implemented",
                    ),
                    unavailable_runtime_variant_capability(
                        "candle",
                        "candle.cuda",
                        InferenceDeviceClass::Cuda,
                        DeviceResolutionDiagnosticCode::MissingRuntimeVariant,
                        "Candle CUDA runtime variant readiness is not reported",
                    ),
                    #[cfg(target_os = "macos")]
                    unavailable_runtime_variant_capability(
                        "candle",
                        "candle.metal",
                        InferenceDeviceClass::Metal,
                        DeviceResolutionDiagnosticCode::MissingRuntimeVariant,
                        "Candle Metal runtime variant readiness is not reported",
                    ),
                ],
            },
        }
    }

    /// Check if Candle/CUDA is available on the system
    pub fn check_availability() -> (bool, Option<String>) {
        #[cfg(feature = "backend-candle")]
        {
            (
                false,
                Some(
                    "Candle backend has a staged embedding load planner but executable model loading is not implemented"
                        .to_string(),
                ),
            )
        }

        #[cfg(not(feature = "backend-candle"))]
        {
            (
                false,
                Some("Candle feature not enabled at compile time".to_string()),
            )
        }
    }

    /// Validate a Pumas package for the staged Candle embedding loader and
    /// project it into the shared backend-load model-source contract.
    pub fn embedding_model_source_from_package(
        package: &ResolvedModelPackageFacts,
    ) -> Result<ResolvedModelSource, BackendError> {
        if !package.uses_current_contract() {
            return Err(BackendError::Config(format!(
                "Candle package facts contract version {} is unsupported",
                package.package_facts_contract_version
            )));
        }
        if matches!(
            package.artifact.validation_state,
            ModelValidationState::Invalid | ModelValidationState::Unknown
        ) {
            return Err(BackendError::Config(
                "Candle package artifact is not valid".to_string(),
            ));
        }
        if !matches!(
            package.artifact.artifact_kind,
            ModelArtifactKind::HfCompatibleDirectory
        ) {
            return Err(BackendError::Config(format!(
                "Candle embedding cannot load {:?} artifacts; expected an HF-compatible directory containing safetensors weights and tokenizer files",
                package.artifact.artifact_kind
            )));
        }

        let task =
            resolve_task_registry_entry_from_evidence(&package.task).map_err(|diagnostic| {
                BackendError::Config(format!(
                    "Candle package task evidence is not loadable: {}",
                    diagnostic.message
                ))
            })?;
        if task.task_id != InferenceTaskId::Embedding {
            return Err(BackendError::Config(format!(
                "Candle staged loader only accepts embedding packages, got '{}'",
                task.canonical_label()
            )));
        }
        if package.custom_code.requires_custom_code {
            return Err(BackendError::Config(
                "Candle staged loader does not execute custom model code".to_string(),
            ));
        }
        if !has_present_component(package, ProcessorComponentKind::Config) {
            return Err(BackendError::Config(
                "Candle embedding package requires a present config component".to_string(),
            ));
        }
        if !has_present_component(package, ProcessorComponentKind::Weights) {
            return Err(BackendError::Config(
                "Candle embedding package requires present safetensors weights".to_string(),
            ));
        }
        if !has_present_component(package, ProcessorComponentKind::Tokenizer) {
            return Err(BackendError::Config(
                "Candle embedding package requires a present tokenizer component".to_string(),
            ));
        }

        let source = ResolvedModelSource::from_package_facts(package);
        source.validate_for_backend_load().map_err(|diagnostics| {
            let codes = diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            BackendError::Config(format!("Invalid Candle model source: {codes}"))
        })?;
        Ok(source)
    }

    /// Resolve a factual Candle embedding load plan from Pumas package facts.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError::Config`] when the package facts are not a
    /// supported embedding HF-compatible directory, when required package files
    /// are missing, or when dtype/model/device hints are outside the first
    /// staged Candle slice.
    pub fn embedding_load_plan_from_package(
        package: &ResolvedModelPackageFacts,
        device_hint: Option<&str>,
    ) -> Result<CandleEmbeddingLoadPlan, BackendError> {
        let source = Self::embedding_model_source_from_package(package)?;
        let transformers = package.transformers.as_ref().ok_or_else(|| {
            BackendError::Config(
                "Candle embedding package requires Transformers package evidence".to_string(),
            )
        })?;
        let model_type = transformers
            .config_model_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                BackendError::Config(
                    "Candle embedding package requires config_model_type evidence".to_string(),
                )
            })?
            .to_ascii_lowercase();
        if model_type != "bert" {
            return Err(BackendError::Config(format!(
                "Candle staged embedding loader only supports bert model_type, got '{model_type}'"
            )));
        }

        let model_dir = existing_directory(&source.entry_path, "Candle model directory")?;
        let config_path = existing_component_file(
            package,
            &model_dir,
            ProcessorComponentKind::Config,
            "config",
        )?;
        let tokenizer_path = existing_component_file(
            package,
            &model_dir,
            ProcessorComponentKind::Tokenizer,
            "tokenizer",
        )?;
        let safetensors_path = existing_component_file(
            package,
            &model_dir,
            ProcessorComponentKind::Weights,
            "weights",
        )?;
        if safetensors_path
            .extension()
            .and_then(|value| value.to_str())
            != Some("safetensors")
        {
            return Err(BackendError::Config(format!(
                "Candle staged embedding loader requires safetensors weights, got {}",
                safetensors_path.display()
            )));
        }

        Ok(CandleEmbeddingLoadPlan {
            source,
            model_dir,
            config_path,
            tokenizer_path,
            safetensors_path,
            dtype: candle_dtype_from_transformers(transformers)?,
            device: candle_device_from_hint(device_hint)?,
            model_type,
            architecture: transformers.architectures.first().cloned(),
        })
    }

    /// Load concrete Candle resources from a staged embedding load plan.
    ///
    /// The resource probe intentionally remains separate from backend
    /// availability and runtime startup. It proves that the plan can be
    /// consumed by Candle/tokenizers APIs without advertising executable model
    /// support before a model module and execution path exist.
    #[cfg(feature = "backend-candle")]
    pub fn embedding_load_resources_from_plan(
        plan: &CandleEmbeddingLoadPlan,
    ) -> Result<CandleEmbeddingLoadResources, BackendError> {
        let device = candle_device_from_load_plan(plan.device)?;
        let dtype = candle_core_dtype_from_load_plan(plan.dtype);
        let tokenizer =
            tokenizers::Tokenizer::from_file(&plan.tokenizer_path).map_err(|error| {
                BackendError::Config(format!(
                    "Candle tokenizer load failed for {}: {error}",
                    plan.tokenizer_path.display()
                ))
            })?;
        let tensors =
            candle_core::safetensors::load(&plan.safetensors_path, &device).map_err(|error| {
                BackendError::Config(format!(
                    "Candle safetensors load failed for {}: {error}",
                    plan.safetensors_path.display()
                ))
            })?;
        if tensors.is_empty() {
            return Err(BackendError::Config(format!(
                "Candle safetensors file contains no tensors: {}",
                plan.safetensors_path.display()
            )));
        }

        Ok(CandleEmbeddingLoadResources {
            device,
            dtype,
            tokenizer,
            tensors,
        })
    }
}

fn has_present_component(
    package: &ResolvedModelPackageFacts,
    kind: ProcessorComponentKind,
) -> bool {
    package
        .components
        .iter()
        .any(|component| component.kind == kind && component.status == PackageFactStatus::Present)
}

fn existing_directory(raw_path: &str, label: &str) -> Result<PathBuf, BackendError> {
    let path = PathBuf::from(raw_path);
    if path.is_dir() {
        Ok(path)
    } else {
        Err(BackendError::Config(format!(
            "{label} does not exist or is not a directory: {}",
            path.display()
        )))
    }
}

fn existing_component_file(
    package: &ResolvedModelPackageFacts,
    model_dir: &Path,
    kind: ProcessorComponentKind,
    label: &str,
) -> Result<PathBuf, BackendError> {
    let relative_path = package
        .components
        .iter()
        .find(|component| component.kind == kind && component.status == PackageFactStatus::Present)
        .and_then(|component| component.relative_path.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            BackendError::Config(format!(
                "Candle embedding package requires a present {label} component path"
            ))
        })?;
    let relative_path = safe_relative_component_path(relative_path).ok_or_else(|| {
        BackendError::Config(format!(
            "Candle embedding package has an unsafe {label} component path: {relative_path}"
        ))
    })?;
    let path = model_dir.join(relative_path);
    if path.is_file() {
        Ok(path)
    } else {
        Err(BackendError::Config(format!(
            "Candle embedding package {label} file is missing: {}",
            path.display()
        )))
    }
}

fn safe_relative_component_path(raw_path: &str) -> Option<PathBuf> {
    let path = Path::new(raw_path);
    if path.is_absolute() {
        return None;
    }

    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => clean.push(value),
            _ => return None,
        }
    }

    if clean.as_os_str().is_empty() {
        None
    } else {
        Some(clean)
    }
}

fn candle_dtype_from_transformers(
    transformers: &crate::model_contracts::TransformersPackageEvidence,
) -> Result<CandleLoadDType, BackendError> {
    let dtype = transformers
        .torch_dtype
        .as_deref()
        .or(transformers.dtype.as_deref())
        .unwrap_or("float32")
        .trim()
        .to_ascii_lowercase()
        .replace('_', "");
    match dtype.as_str() {
        "float32" | "f32" => Ok(CandleLoadDType::F32),
        "float16" | "f16" => Ok(CandleLoadDType::F16),
        "bfloat16" | "bf16" => Ok(CandleLoadDType::BF16),
        _ => Err(BackendError::Config(format!(
            "Candle staged embedding loader does not support dtype '{dtype}'"
        ))),
    }
}

fn candle_device_from_hint(device_hint: Option<&str>) -> Result<CandleLoadDevice, BackendError> {
    let Some(device_hint) = device_hint.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(CandleLoadDevice::Auto);
    };
    let normalized = device_hint.to_ascii_lowercase();
    if normalized == "auto" {
        return Ok(CandleLoadDevice::Auto);
    }
    if normalized == "cpu" {
        return Ok(CandleLoadDevice::Cpu);
    }
    if normalized == "cuda" {
        return Ok(CandleLoadDevice::Cuda { index: 0 });
    }
    if let Some(index) = normalized.strip_prefix("cuda:") {
        let index = index.parse::<usize>().map_err(|_| {
            BackendError::Config(format!(
                "Candle staged embedding loader does not support device hint '{device_hint}'"
            ))
        })?;
        return Ok(CandleLoadDevice::Cuda { index });
    }

    Err(BackendError::Config(format!(
        "Candle staged embedding loader does not support device hint '{device_hint}'"
    )))
}

#[cfg(feature = "backend-candle")]
fn candle_core_dtype_from_load_plan(dtype: CandleLoadDType) -> candle_core::DType {
    match dtype {
        CandleLoadDType::F32 => candle_core::DType::F32,
        CandleLoadDType::F16 => candle_core::DType::F16,
        CandleLoadDType::BF16 => candle_core::DType::BF16,
    }
}

#[cfg(feature = "backend-candle")]
fn candle_device_from_load_plan(
    device: CandleLoadDevice,
) -> Result<candle_core::Device, BackendError> {
    match device {
        CandleLoadDevice::Auto | CandleLoadDevice::Cpu => Ok(candle_core::Device::Cpu),
        CandleLoadDevice::Cuda { index } => candle_core::Device::new_cuda(index).map_err(|error| {
            BackendError::Config(format!(
                "Candle CUDA device {index} could not be initialized: {error}"
            ))
        }),
    }
}

impl Default for CandleBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for CandleBackend {
    fn name(&self) -> &'static str {
        "Candle"
    }

    fn description(&self) -> &'static str {
        "In-process Candle inference with CUDA support. Optimized for embedding models."
    }

    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Err(BackendError::StartupFailed(
            "Candle backend is staged for embedding-only support, but executable model loading is not implemented".to_string()
        ))
    }

    fn stop(&mut self) {
        self.base_url = None;
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn health_check(&self) -> bool {
        if let Some(ref base_url) = self.base_url {
            let health_url = format!("{}/health", base_url);
            match self.http_client.get(&health_url).send().await {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            }
        } else {
            false
        }
    }

    fn base_url(&self) -> Option<String> {
        self.base_url.clone()
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Err(BackendError::Inference(
            "Chat completion not supported by Candle backend".to_string(),
        ))
    }

    async fn embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        let base_url = self.base_url.as_ref().ok_or(BackendError::NotReady)?;

        let url = format!("{}/v1/embeddings", base_url);

        let request = serde_json::json!({
            "input": texts,
            "model": model,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(BackendError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BackendError::Inference(format!(
                "Embedding API error {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BackendError::Inference(format!("Failed to parse response: {}", e)))?;

        let data = json.get("data").and_then(|d| d.as_array()).ok_or_else(|| {
            BackendError::Inference("Invalid embedding response format".to_string())
        })?;
        let token_count = openai_embedding_token_count_for_single_result(&json, data.len());

        let mut results = Vec::new();
        for item in data {
            let embedding = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| BackendError::Inference("Missing embedding vector".to_string()))?;

            let vector: Vec<f32> = embedding
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();

            results.push(EmbeddingResult {
                vector,
                token_count,
            });
        }

        Ok(results)
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Err(BackendError::Inference(
            "Reranking not supported by Candle backend".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn package_fixture(raw: &str) -> ResolvedModelPackageFacts {
        serde_json::from_str(raw).expect("package facts fixture should decode")
    }

    fn package_fixture_with_entry_path(raw: &str, entry_path: &Path) -> ResolvedModelPackageFacts {
        let mut value: serde_json::Value =
            serde_json::from_str(raw).expect("package facts fixture should decode");
        value["artifact"]["entry_path"] = serde_json::json!(entry_path.display().to_string());
        serde_json::from_value(value).expect("package facts fixture should decode")
    }

    fn write_minimal_candle_package_files(model_dir: &Path) {
        fs::write(model_dir.join("config.json"), "{}").expect("config fixture should write");
        fs::write(model_dir.join("tokenizer.json"), "{}").expect("tokenizer fixture should write");
        fs::write(model_dir.join("model.safetensors"), b"fixture")
            .expect("safetensors fixture should write");
    }

    #[cfg(feature = "backend-candle")]
    fn write_valid_candle_resource_files(model_dir: &Path) {
        use std::collections::HashMap;

        fs::write(model_dir.join("config.json"), "{}").expect("config fixture should write");

        let vocab = HashMap::from([
            ("[UNK]".to_string(), 0),
            ("hello".to_string(), 1),
            ("world".to_string(), 2),
        ]);
        let word_level = tokenizers::models::wordlevel::WordLevel::builder()
            .vocab(vocab)
            .unk_token("[UNK]".to_string())
            .build()
            .expect("word-level tokenizer fixture should build");
        let tokenizer = tokenizers::Tokenizer::new(word_level);
        tokenizer
            .save(model_dir.join("tokenizer.json"), false)
            .expect("tokenizer fixture should save");

        let tensor = candle_core::Tensor::from_slice(
            &[0.0f32, 1.0, 2.0, 3.0],
            (2, 2),
            &candle_core::Device::Cpu,
        )
        .expect("tensor fixture should build");
        tensor
            .save_safetensors(
                "embeddings.word_embeddings.weight",
                model_dir.join("model.safetensors"),
            )
            .expect("safetensors fixture should save");
    }

    #[test]
    fn test_backend_name() {
        let backend = CandleBackend::new();
        assert_eq!(backend.name(), "Candle");
    }

    #[test]
    fn test_capabilities() {
        let caps = CandleBackend::static_capabilities();
        assert!(!caps.vision);
        assert!(caps.embeddings);
        assert!(caps.gpu);
        assert!(!caps.streaming);
        assert!(caps.supports_task(InferenceTaskId::Embedding));
        assert!(!caps.supports_task(InferenceTaskId::TextGeneration));
        assert!(caps.facts.runtime_variants.iter().any(|variant| {
            variant.runtime_variant_id.as_str() == "candle.cpu"
                && variant.device_class == InferenceDeviceClass::Cpu
                && !variant.available
        }));
        assert!(caps.facts.runtime_variants.iter().any(|variant| {
            variant.runtime_variant_id.as_str() == "candle.cuda"
                && variant.device_class == InferenceDeviceClass::Cuda
                && !variant.available
        }));
        assert_eq!(
            caps.facts.features.kv_cache,
            BackendFeatureSupport::Unsupported
        );
    }

    #[test]
    fn test_not_ready_initially() {
        let backend = CandleBackend::new();
        assert!(!backend.is_ready());
        assert!(backend.base_url().is_none());
    }

    #[cfg(feature = "backend-candle")]
    #[test]
    fn test_staged_backend_reports_unavailable_until_loader_exists() {
        let (available, reason) = CandleBackend::check_availability();

        assert!(!available);
        assert!(reason
            .as_deref()
            .is_some_and(|value| value.contains("executable model loading is not implemented")));
    }

    #[test]
    fn embedding_model_source_accepts_hf_embedding_package_facts() {
        let package = package_fixture(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
        ));

        let source = CandleBackend::embedding_model_source_from_package(&package)
            .expect("Candle embedding package facts should map to model source");

        assert_eq!(
            source.artifact_kind,
            ModelArtifactKind::HfCompatibleDirectory
        );
        assert_eq!(source.entry_path, package.artifact.entry_path);
        assert_eq!(source.model_ref, Some(package.model_ref));
        source
            .validate_for_backend_load()
            .expect("mapped source should remain backend-load valid");
    }

    #[test]
    fn embedding_load_plan_resolves_candle_paths_dtype_and_device() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        write_minimal_candle_package_files(temp.path());
        let package = package_fixture_with_entry_path(
            include_str!(
                "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
            ),
            temp.path(),
        );

        let plan = CandleBackend::embedding_load_plan_from_package(&package, Some("cuda:1"))
            .expect("Candle package facts should resolve to a load plan");

        assert_eq!(plan.model_dir, temp.path());
        assert_eq!(plan.config_path, temp.path().join("config.json"));
        assert_eq!(plan.tokenizer_path, temp.path().join("tokenizer.json"));
        assert_eq!(plan.safetensors_path, temp.path().join("model.safetensors"));
        assert_eq!(plan.dtype, CandleLoadDType::F32);
        assert_eq!(plan.device, CandleLoadDevice::Cuda { index: 1 });
        assert_eq!(plan.model_type, "bert");
        assert_eq!(plan.architecture.as_deref(), Some("BertModel"));
        assert_eq!(
            plan.source.artifact_kind,
            ModelArtifactKind::HfCompatibleDirectory
        );
    }

    #[cfg(feature = "backend-candle")]
    #[test]
    fn embedding_load_resources_loads_tokenizer_safetensors_dtype_and_cpu_device() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        write_valid_candle_resource_files(temp.path());
        let package = package_fixture_with_entry_path(
            include_str!(
                "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
            ),
            temp.path(),
        );
        let plan = CandleBackend::embedding_load_plan_from_package(&package, Some("cpu"))
            .expect("Candle package facts should resolve to a load plan");

        let resources = CandleBackend::embedding_load_resources_from_plan(&plan)
            .expect("Candle resources should load from valid staged plan");

        assert!(matches!(resources.device, candle_core::Device::Cpu));
        assert_eq!(resources.dtype, candle_core::DType::F32);
        assert_eq!(resources.tokenizer.get_vocab_size(false), 3);
        assert_eq!(resources.tensors.len(), 1);
        assert!(resources
            .tensors
            .contains_key("embeddings.word_embeddings.weight"));
    }

    #[cfg(feature = "backend-candle")]
    #[test]
    fn embedding_load_resources_rejects_invalid_tokenizer_or_safetensors() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        write_valid_candle_resource_files(temp.path());
        fs::write(temp.path().join("tokenizer.json"), "{}")
            .expect("invalid tokenizer fixture should write");
        let package = package_fixture_with_entry_path(
            include_str!(
                "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
            ),
            temp.path(),
        );
        let plan = CandleBackend::embedding_load_plan_from_package(&package, Some("cpu"))
            .expect("Candle package facts should resolve to a load plan");

        let error = CandleBackend::embedding_load_resources_from_plan(&plan)
            .expect_err("Candle resources should reject invalid tokenizer files");

        assert!(error.to_string().contains("tokenizer load failed"));

        write_valid_candle_resource_files(temp.path());
        fs::write(
            temp.path().join("model.safetensors"),
            b"not a safetensors file",
        )
        .expect("invalid safetensors fixture should write");
        let package = package_fixture_with_entry_path(
            include_str!(
                "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
            ),
            temp.path(),
        );
        let plan = CandleBackend::embedding_load_plan_from_package(&package, Some("cpu"))
            .expect("Candle package facts should resolve to a load plan");

        let error = CandleBackend::embedding_load_resources_from_plan(&plan)
            .expect_err("Candle resources should reject invalid safetensors files");

        assert!(error.to_string().contains("safetensors load failed"));
    }

    #[test]
    fn embedding_load_plan_rejects_missing_package_files() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        fs::write(temp.path().join("config.json"), "{}").expect("config fixture should write");
        fs::write(temp.path().join("model.safetensors"), b"fixture")
            .expect("safetensors fixture should write");
        let package = package_fixture_with_entry_path(
            include_str!(
                "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
            ),
            temp.path(),
        );

        let error = CandleBackend::embedding_load_plan_from_package(&package, Some("cpu"))
            .expect_err("Candle load plan should fail closed on missing tokenizer file");

        assert!(error.to_string().contains("tokenizer file is missing"));
    }

    #[test]
    fn embedding_load_plan_rejects_unsafe_component_paths() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        write_minimal_candle_package_files(temp.path());
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
        ))
        .expect("fixture json should decode");
        value["artifact"]["entry_path"] = serde_json::json!(temp.path().display().to_string());
        value["components"][1]["relative_path"] = serde_json::json!("../model.safetensors");
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value).expect("fixture should decode");

        let error = CandleBackend::embedding_load_plan_from_package(&package, None)
            .expect_err("Candle load plan should fail closed on unsafe component paths");

        assert!(error.to_string().contains("unsafe weights component path"));
    }

    #[test]
    fn embedding_load_plan_rejects_unsupported_dtype_model_type_and_device() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        write_minimal_candle_package_files(temp.path());
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
        ))
        .expect("fixture json should decode");
        value["artifact"]["entry_path"] = serde_json::json!(temp.path().display().to_string());
        value["transformers"]["torch_dtype"] = serde_json::json!("int8");
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value.clone()).expect("fixture should decode");
        let error = CandleBackend::embedding_load_plan_from_package(&package, None)
            .expect_err("Candle load plan should reject unsupported dtype");
        assert!(error.to_string().contains("does not support dtype 'int8'"));

        value["transformers"]["torch_dtype"] = serde_json::json!("float32");
        value["transformers"]["config_model_type"] = serde_json::json!("llama");
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value.clone()).expect("fixture should decode");
        let error = CandleBackend::embedding_load_plan_from_package(&package, None)
            .expect_err("Candle load plan should reject unsupported model type");
        assert!(error.to_string().contains("only supports bert model_type"));

        value["transformers"]["config_model_type"] = serde_json::json!("bert");
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value).expect("fixture should decode");
        let error = CandleBackend::embedding_load_plan_from_package(&package, Some("vulkan:0"))
            .expect_err("Candle load plan should reject unsupported device hints");
        assert!(error
            .to_string()
            .contains("does not support device hint 'vulkan:0'"));
    }

    #[test]
    fn embedding_model_source_rejects_gguf_packages() {
        let package = package_fixture(include_str!(
            "../../tests/fixtures/inference_package_facts/gguf_embedding_package_facts.json"
        ));

        let error = CandleBackend::embedding_model_source_from_package(&package)
            .expect_err("Candle should not accept GGUF packages");

        assert!(error
            .to_string()
            .contains("Candle embedding cannot load Gguf artifacts"));
    }

    #[test]
    fn embedding_model_source_rejects_non_embedding_task_facts() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
        ))
        .expect("fixture json should decode");
        value["custom_code"] = serde_json::json!({
            "requires_custom_code": false
        });
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value).expect("fixture should decode");

        let error = CandleBackend::embedding_model_source_from_package(&package)
            .expect_err("Candle staged loader should only accept embedding tasks");

        assert!(error
            .to_string()
            .contains("only accepts embedding packages"));
    }

    #[test]
    fn embedding_model_source_rejects_missing_tokenizer() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
        ))
        .expect("fixture json should decode");
        value["components"] = serde_json::json!([
            {
                "kind": "config",
                "status": "present",
                "relative_path": "config.json"
            },
            {
                "kind": "weights",
                "status": "present",
                "relative_path": "model.safetensors"
            }
        ]);
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value).expect("fixture should decode");

        let error = CandleBackend::embedding_model_source_from_package(&package)
            .expect_err("Candle should require a present tokenizer");

        assert!(error
            .to_string()
            .contains("requires a present tokenizer component"));
    }

    #[test]
    fn embedding_model_source_rejects_missing_config() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
        ))
        .expect("fixture json should decode");
        value["components"] = serde_json::json!([
            {
                "kind": "weights",
                "status": "present",
                "relative_path": "model.safetensors"
            },
            {
                "kind": "tokenizer",
                "status": "present",
                "relative_path": "tokenizer.json"
            }
        ]);
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value).expect("fixture should decode");

        let error = CandleBackend::embedding_model_source_from_package(&package)
            .expect_err("Candle should require a present config");

        assert!(error
            .to_string()
            .contains("requires a present config component"));
    }

    #[test]
    fn embedding_model_source_rejects_missing_weights() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
        ))
        .expect("fixture json should decode");
        value["components"] = serde_json::json!([
            {
                "kind": "config",
                "status": "present",
                "relative_path": "config.json"
            },
            {
                "kind": "tokenizer",
                "status": "present",
                "relative_path": "tokenizer.json"
            }
        ]);
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value).expect("fixture should decode");

        let error = CandleBackend::embedding_model_source_from_package(&package)
            .expect_err("Candle should require present safetensors weights");

        assert!(error
            .to_string()
            .contains("requires present safetensors weights"));
    }

    #[test]
    fn embedding_model_source_rejects_custom_code_packages() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
        ))
        .expect("fixture json should decode");
        value["task"] = serde_json::json!({
            "pipeline_tag": "feature-extraction",
            "task_type_primary": "embedding",
            "input_modalities": ["text"],
            "output_modalities": ["embedding"]
        });
        let package: ResolvedModelPackageFacts =
            serde_json::from_value(value).expect("fixture should decode");

        let error = CandleBackend::embedding_model_source_from_package(&package)
            .expect_err("Candle should fail closed on custom code packages");

        assert!(error
            .to_string()
            .contains("does not execute custom model code"));
    }
}
