use super::*;
use std::fs;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::{Stream, stream};
use inference::backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use inference::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use inference::{RerankRequest, RerankResponse};

pub(super) struct MockKvProcessHandle;

impl ProcessHandle for MockKvProcessHandle {
    fn pid(&self) -> u32 {
        1
    }

    fn kill(&self) -> std::result::Result<(), String> {
        Ok(())
    }
}

pub(super) struct MockKvProcessSpawner;

#[async_trait]
impl ProcessSpawner for MockKvProcessSpawner {
    async fn spawn_sidecar(
        &self,
        _sidecar_name: &str,
        _args: &[&str],
    ) -> std::result::Result<
        (
            tokio::sync::mpsc::Receiver<ProcessEvent>,
            Box<dyn ProcessHandle>,
        ),
        String,
    > {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok((rx, Box::new(MockKvProcessHandle)))
    }

    fn app_data_dir(&self) -> std::result::Result<PathBuf, String> {
        Ok(std::env::temp_dir())
    }

    fn binaries_dir(&self) -> std::result::Result<PathBuf, String> {
        Ok(std::env::temp_dir())
    }
}

pub(super) struct MockKvBackend {
    pub(super) bytes: Vec<u8>,
    pub(super) restored: Arc<Mutex<Vec<Vec<u8>>>>,
}

#[async_trait]
impl InferenceBackend for MockKvBackend {
    fn name(&self) -> &'static str {
        "MockKv"
    }

    fn description(&self) -> &'static str {
        "Mock backend with KV slot support"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> std::result::Result<BackendStartOutcome, BackendError> {
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
    ) -> std::result::Result<
        Pin<Box<dyn Stream<Item = std::result::Result<ChatChunk, BackendError>> + Send>>,
        BackendError,
    > {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> std::result::Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(
        &self,
        _request: RerankRequest,
    ) -> std::result::Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }

    async fn kv_cache_runtime_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> std::result::Result<KvCacheRuntimeFingerprint, BackendError> {
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
    ) -> std::result::Result<ModelFingerprint, BackendError> {
        Ok(ModelFingerprint {
            model_id: "model".to_string(),
            config_hash: "cfg".to_string(),
        })
    }

    async fn save_kv_cache_slot(
        &self,
        _slot_id: u32,
        path: &Path,
    ) -> std::result::Result<(), BackendError> {
        fs::write(path, &self.bytes)
            .map_err(|error| BackendError::Inference(format!("mock save failed: {}", error)))
    }

    async fn restore_kv_cache_slot(
        &self,
        _slot_id: u32,
        path: &Path,
    ) -> std::result::Result<(), BackendError> {
        let bytes = fs::read(path)
            .map_err(|error| BackendError::Inference(format!("mock restore failed: {}", error)))?;
        self.restored
            .lock()
            .expect("lock should succeed")
            .push(bytes);
        Ok(())
    }

    async fn truncate_kv_cache_data(
        &self,
        data: &[u8],
        token_position: usize,
        _active_config: Option<&BackendConfig>,
    ) -> std::result::Result<Vec<u8>, BackendError> {
        Ok(data[..token_position.min(data.len())].to_vec())
    }
}
