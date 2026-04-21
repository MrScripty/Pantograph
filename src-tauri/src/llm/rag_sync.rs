//! Host-side synchronization between gateway embedding state and the RAG manager.
//!
//! This keeps Tauri-owned consumers aligned with the backend-owned gateway mode
//! and embedding runtime availability without moving runtime policy into the
//! host layer.

use crate::agent::rag::SharedRagManager;

use super::SharedGateway;

/// Synchronize the RAG manager's embedding endpoint from the shared gateway.
///
/// Returns the embedding URL that is now available for RAG consumers, or
/// `None` when no embedding-capable runtime is currently exposed by the host.
pub async fn sync_rag_embedding_url_from_gateway(
    gateway: &SharedGateway,
    rag_manager: &SharedRagManager,
) -> Option<String> {
    let embedding_url = gateway.embedding_url().await;
    let mut rag = rag_manager.write().await;

    if let Some(url) = embedding_url.clone() {
        rag.set_embedding_url(url);
    } else {
        rag.clear_embedding_url();
    }

    embedding_url
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use futures_util::stream;
    use futures_util::Stream;
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        InferenceBackend,
    };
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::{ImageGenerationRequest, ImageGenerationResult, RerankRequest, RerankResponse};
    use tokio::sync::mpsc;

    use crate::agent::rag::{create_rag_manager, SharedRagManager};
    use crate::llm::gateway::InferenceGateway;

    use super::sync_rag_embedding_url_from_gateway;

    struct MockProcessSpawner;

    #[async_trait]
    impl ProcessSpawner for MockProcessSpawner {
        async fn spawn_sidecar(
            &self,
            _sidecar_name: &str,
            _args: &[&str],
        ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
            Err("spawn should not be called in rag sync tests".to_string())
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }
    }

    struct MockInferenceBackend {
        ready: Arc<Mutex<bool>>,
    }

    impl MockInferenceBackend {
        fn new() -> Self {
            Self {
                ready: Arc::new(Mutex::new(false)),
            }
        }
    }

    #[async_trait]
    impl InferenceBackend for MockInferenceBackend {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn description(&self) -> &'static str {
            "Mock backend for rag sync tests"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            *self.ready.lock().expect("mock backend ready lock poisoned") = true;
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            })
        }

        fn stop(&mut self) {
            *self.ready.lock().expect("mock backend ready lock poisoned") = false;
        }

        fn is_ready(&self) -> bool {
            *self.ready.lock().expect("mock backend ready lock poisoned")
        }

        async fn health_check(&self) -> bool {
            self.is_ready()
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
        ) -> Result<Vec<inference::EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
            Err(BackendError::Inference(
                "rerank should not be called in rag sync tests".to_string(),
            ))
        }

        async fn generate_image(
            &self,
            _request: ImageGenerationRequest,
        ) -> Result<ImageGenerationResult, BackendError> {
            Err(BackendError::Inference(
                "image generation should not be called in rag sync tests".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn sync_rag_embedding_url_clears_stale_url_when_gateway_has_no_embedding_runtime() {
        let temp = tempfile::tempdir().expect("tempdir");
        let rag_manager: SharedRagManager = create_rag_manager(temp.path().to_path_buf());
        rag_manager
            .write()
            .await
            .set_embedding_url("http://127.0.0.1:9999".to_string());

        let gateway = Arc::new(InferenceGateway::with_test_backend(
            Box::new(MockInferenceBackend::new()),
            "mock",
            Arc::new(MockProcessSpawner),
        ));
        gateway.init().await;

        let embedding_url = sync_rag_embedding_url_from_gateway(&gateway, &rag_manager).await;
        let status = rag_manager.read().await.status().clone();

        assert_eq!(embedding_url, None);
        assert_eq!(status.vectorizer_url, None);
        assert!(!status.vectorizer_available);
    }

    #[tokio::test]
    async fn sync_rag_embedding_url_uses_gateway_embedding_mode_url() {
        let temp = tempfile::tempdir().expect("tempdir");
        let rag_manager: SharedRagManager = create_rag_manager(temp.path().to_path_buf());

        let gateway = Arc::new(InferenceGateway::with_test_backend(
            Box::new(MockInferenceBackend::new()),
            "mock",
            Arc::new(MockProcessSpawner),
        ));
        gateway.init().await;
        gateway
            .start(&BackendConfig {
                embedding_mode: true,
                ..Default::default()
            })
            .await
            .expect("gateway should start in embedding mode");

        let embedding_url = sync_rag_embedding_url_from_gateway(&gateway, &rag_manager).await;
        let status = rag_manager.read().await.status().clone();

        assert_eq!(embedding_url.as_deref(), Some("http://127.0.0.1:11434"));
        assert_eq!(
            status.vectorizer_url.as_deref(),
            Some("http://127.0.0.1:11434")
        );
    }
}
