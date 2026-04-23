use super::*;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::stream;
use tokio::sync::mpsc;

use crate::backend::BackendStartOutcome;

#[path = "gateway_tests/start_config.rs"]
mod start_config;

struct MockImageBackend;
struct MockHttpBackend;
struct MockReusedBackend;
struct MockImplicitLifecycleBackend;
struct MockFailingBackend;
struct MockKvBackend;

struct MockProcessHandle;

impl crate::process::ProcessHandle for MockProcessHandle {
    fn pid(&self) -> u32 {
        1
    }

    fn kill(&self) -> Result<(), String> {
        Ok(())
    }
}

struct MockProcessSpawner;

#[async_trait]
impl ProcessSpawner for MockProcessSpawner {
    async fn spawn_sidecar(
        &self,
        _sidecar_name: &str,
        _args: &[&str],
    ) -> Result<
        (
            mpsc::Receiver<crate::process::ProcessEvent>,
            Box<dyn crate::process::ProcessHandle>,
        ),
        String,
    > {
        let (_tx, rx) = mpsc::channel(1);
        Ok((rx, Box::new(MockProcessHandle)))
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/tmp"))
    }

    fn binaries_dir(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/tmp"))
    }
}

#[async_trait]
impl InferenceBackend for MockImageBackend {
    fn name(&self) -> &'static str {
        "Mock"
    }

    fn description(&self) -> &'static str {
        "Mock image backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            image_generation: true,
            external_connection: true,
            ..BackendCapabilities::default()
        }
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("started_mock_runtime".to_string()),
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }

    async fn generate_image(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, BackendError> {
        Ok(ImageGenerationResult {
            images: vec![crate::types::EncodedImage {
                data_base64: request.prompt,
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(512),
            }],
            seed_used: Some(7),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockHttpBackend {
    fn name(&self) -> &'static str {
        "MockHttp"
    }

    fn description(&self) -> &'static str {
        "Mock HTTP backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            external_connection: true,
            ..BackendCapabilities::default()
        }
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("started_http_runtime".to_string()),
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        Some("http://127.0.0.1:11434".to_string())
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }

    async fn generate_image(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, BackendError> {
        Ok(ImageGenerationResult {
            images: vec![crate::types::EncodedImage {
                data_base64: request.prompt,
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(512),
            }],
            seed_used: Some(11),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockReusedBackend {
    fn name(&self) -> &'static str {
        "MockReused"
    }

    fn description(&self) -> &'static str {
        "Mock reused backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("reused_mock_runtime".to_string()),
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockImplicitLifecycleBackend {
    fn name(&self) -> &'static str {
        "MockImplicitLifecycle"
    }

    fn description(&self) -> &'static str {
        "Mock backend without explicit lifecycle reasons"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: None,
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockFailingBackend {
    fn name(&self) -> &'static str {
        "MockFailing"
    }

    fn description(&self) -> &'static str {
        "Mock backend that fails to start"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Err(BackendError::StartupFailed(
            "mock start failure".to_string(),
        ))
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        false
    }

    async fn health_check(&self) -> bool {
        false
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockKvBackend {
    fn name(&self) -> &'static str {
        "MockKv"
    }

    fn description(&self) -> &'static str {
        "Mock backend with KV support"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome::default())
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        Some("http://127.0.0.1:11434".to_string())
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }

    async fn kv_cache_runtime_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<KvCacheRuntimeFingerprint, BackendError> {
        Ok(KvCacheRuntimeFingerprint {
            runtime_id: "mock".to_string(),
            backend_key: "mock".to_string(),
            tokenizer_fingerprint: "tok".to_string(),
            prompt_format_fingerprint: Some("prompt".to_string()),
            runtime_build_fingerprint: Some("build".to_string()),
        })
    }

    async fn kv_cache_model_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<ModelFingerprint, BackendError> {
        Ok(ModelFingerprint {
            model_id: "model".to_string(),
            config_hash: "cfg".to_string(),
        })
    }

    async fn save_kv_cache_slot(
        &self,
        _slot_id: u32,
        _path: &std::path::Path,
    ) -> Result<(), BackendError> {
        Ok(())
    }

    async fn restore_kv_cache_slot(
        &self,
        _slot_id: u32,
        _path: &std::path::Path,
    ) -> Result<(), BackendError> {
        Ok(())
    }

    async fn clear_kv_cache_slot(&self, _slot_id: u32) -> Result<(), BackendError> {
        Ok(())
    }

    async fn truncate_kv_cache_data(
        &self,
        data: &[u8],
        token_position: usize,
        _active_config: Option<&BackendConfig>,
    ) -> Result<Vec<u8>, BackendError> {
        Ok(data[..token_position.min(data.len())].to_vec())
    }
}

#[cfg(feature = "backend-llamacpp")]
#[test]
fn test_gateway_creation() {
    let gateway = InferenceGateway::new();
    // Registry should have at least llama.cpp
    assert!(!gateway.registry.list().is_empty());
}

#[cfg(feature = "backend-llamacpp")]
#[tokio::test]
async fn test_initial_backend_is_llamacpp() {
    let gateway = InferenceGateway::new();
    let name = gateway.current_backend_name().await;
    assert_eq!(name, "llama.cpp");
}

#[cfg(feature = "backend-llamacpp")]
#[tokio::test]
async fn test_switch_backend_normalizes_llamacpp_alias() {
    let gateway = InferenceGateway::new();

    gateway
        .switch_backend("llama_cpp")
        .await
        .expect("llama_cpp alias should resolve");

    assert_eq!(gateway.current_backend_name().await, "llama.cpp");
    assert_eq!(
        gateway
            .runtime_lifecycle_snapshot()
            .await
            .runtime_id
            .as_deref(),
        Some("llama_cpp")
    );
}

#[cfg(feature = "backend-pytorch")]
#[tokio::test]
async fn test_switch_backend_normalizes_pytorch_alias() {
    let gateway = InferenceGateway::new();

    gateway
        .switch_backend("pytorch")
        .await
        .expect("pytorch alias should resolve");

    assert_eq!(gateway.current_backend_name().await, "PyTorch");
    assert_eq!(
        gateway
            .runtime_lifecycle_snapshot()
            .await
            .runtime_id
            .as_deref(),
        Some("pytorch")
    );
}

#[cfg(feature = "backend-llamacpp")]
#[tokio::test]
async fn test_not_ready_initially() {
    let gateway = InferenceGateway::new();
    assert!(!gateway.is_ready().await);
}

#[tokio::test]
async fn test_generate_image_forwards_to_active_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let result = gateway
        .generate_image(ImageGenerationRequest {
            model: "mock".to_string(),
            prompt: "paper lantern".to_string(),
            negative_prompt: None,
            width: Some(512),
            height: Some(512),
            num_inference_steps: Some(20),
            guidance_scale: Some(4.0),
            seed: Some(7),
            scheduler: None,
            num_images_per_prompt: Some(1),
            init_image: None,
            mask_image: None,
            strength: None,
            extra_options: serde_json::Value::Null,
        })
        .await
        .unwrap();

    assert_eq!(result.seed_used, Some(7));
    assert_eq!(result.images.len(), 1);
    assert_eq!(result.images[0].data_base64, "paper lantern");
}

#[tokio::test]
async fn test_rerank_forwards_to_active_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let result = gateway
        .rerank(RerankRequest {
            model: "mock".to_string(),
            query: "alpha".to_string(),
            documents: vec!["a".to_string()],
            top_n: Some(1),
            return_documents: true,
            extra_options: serde_json::Value::Null,
        })
        .await
        .expect("rerank should forward");
    assert!(result.results.is_empty());
}

#[tokio::test]
async fn test_runtime_lifecycle_snapshot_tracks_start_and_stop() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let started = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(started.runtime_id.as_deref(), Some("mock"));
    assert!(started.runtime_instance_id.is_some());
    assert!(started.warmup_started_at_ms.is_some());
    assert!(started.warmup_completed_at_ms.is_some());
    assert!(started.warmup_duration_ms.is_some());
    assert_eq!(started.runtime_reused, Some(false));
    assert_eq!(
        started.lifecycle_decision_reason.as_deref(),
        Some("started_mock_runtime")
    );
    assert!(started.active);
    assert!(started.last_error.is_none());

    gateway.stop().await;

    let stopped = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(stopped.runtime_id.as_deref(), Some("mock"));
    assert!(!stopped.active);
}

#[tokio::test]
async fn test_runtime_lifecycle_snapshot_preserves_instance_id_for_reused_runtime() {
    let gateway = InferenceGateway::with_backend(Box::new(MockReusedBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");
    let first = gateway.runtime_lifecycle_snapshot().await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should reuse");
    let second = gateway.runtime_lifecycle_snapshot().await;

    assert_eq!(first.runtime_id.as_deref(), Some("mock"));
    assert_eq!(second.runtime_id.as_deref(), Some("mock"));
    assert_eq!(second.runtime_reused, Some(true));
    assert_eq!(second.runtime_instance_id, first.runtime_instance_id);
    assert_eq!(
        second.lifecycle_decision_reason.as_deref(),
        Some("reused_mock_runtime")
    );
}

#[tokio::test]
async fn test_runtime_lifecycle_snapshot_normalizes_missing_start_reason() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImplicitLifecycleBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let snapshot = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(snapshot.runtime_reused, Some(false));
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
}

#[tokio::test]
async fn test_runtime_lifecycle_snapshot_normalizes_start_failure_reason() {
    let gateway = InferenceGateway::with_backend(Box::new(MockFailingBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    let error = gateway.start(&BackendConfig::default()).await;
    assert!(error.is_err());

    let snapshot = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(snapshot.runtime_reused, None);
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_start_failed")
    );
    assert_eq!(
        snapshot.last_error.as_deref(),
        Some("Startup failed: mock start failure")
    );
}

#[tokio::test]
async fn test_mode_info_reports_external_runtime_from_start_config() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig {
            external_url: Some("http://127.0.0.1:1234".to_string()),
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start");

    let mode = gateway.mode_info().await;
    assert_eq!(mode.backend_name.as_deref(), Some("mock"));
    assert_eq!(mode.backend_key.as_deref(), Some("mock"));
    assert_eq!(mode.mode, "external");
    assert!(!mode.is_embedding_mode);
    assert_eq!(
        mode.active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_id.as_deref()),
        Some("mock")
    );
    assert_eq!(
        mode.active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_reused),
        Some(false)
    );
    assert_eq!(mode.embedding_runtime, None);
}

#[tokio::test]
async fn test_mode_info_preserves_selected_backend_after_stop() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");
    gateway.stop().await;

    let mode = gateway.mode_info().await;

    assert_eq!(mode.backend_name.as_deref(), Some("mock"));
    assert_eq!(mode.backend_key.as_deref(), Some("mock"));
    assert!(mode.active_runtime.is_some());
}

#[tokio::test]
async fn test_kv_gateway_methods_delegate_to_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockKvBackend), "mock-kv");

    let runtime = gateway
        .kv_cache_runtime_fingerprint()
        .await
        .expect("runtime fingerprint should be available");
    assert_eq!(runtime.runtime_id, "mock");

    let model = gateway
        .kv_cache_model_fingerprint()
        .await
        .expect("model fingerprint should be available");
    assert_eq!(model.model_id, "model");

    let path = std::path::Path::new("/tmp/mock-slot.bin");
    gateway
        .save_kv_cache_slot(0, path)
        .await
        .expect("save should delegate to backend");
    gateway
        .restore_kv_cache_slot(0, path)
        .await
        .expect("restore should delegate to backend");
    gateway
        .clear_kv_cache_slot(0)
        .await
        .expect("clear should delegate to backend");
    let truncated = gateway
        .truncate_kv_cache_data(&[1, 2, 3, 4], 2)
        .await
        .expect("truncate should delegate to backend");
    assert_eq!(truncated, vec![1, 2]);
}

#[tokio::test]
async fn test_mode_info_reports_active_model_target() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Ollama");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig {
            model_name: Some("llava:13b".to_string()),
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start");

    let mode = gateway.mode_info().await;

    assert_eq!(mode.backend_name.as_deref(), Some("Ollama"));
    assert_eq!(mode.backend_key.as_deref(), Some("ollama"));
    assert_eq!(mode.active_model_target.as_deref(), Some("llava:13b"));
    assert_eq!(mode.embedding_model_target, None);
}
