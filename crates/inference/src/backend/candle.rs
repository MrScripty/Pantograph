//! Candle backend implementation
//!
//! This backend provides in-process inference using Hugging Face Candle.
//! It supports CUDA acceleration and various model architectures.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;

use super::{
    BackendCapabilities, BackendCapabilityFacts, BackendComponentCapability, BackendConfig,
    BackendError, BackendFeatureCapabilityFacts, BackendFeatureSupport,
    BackendModelSourceCapabilityFacts, BackendStartOutcome, BackendTaskCapability, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
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
                    "Candle backend is staged but executable model loading is not implemented"
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
                token_count: 0,
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

    fn package_fixture(raw: &str) -> ResolvedModelPackageFacts {
        serde_json::from_str(raw).expect("package facts fixture should decode")
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
