use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;
use inference::backend::BackendStartOutcome;
use inference::{
    BackendCapabilities, BackendConfig, BackendError, ChatChunk, EmbeddingResult, InferenceBackend,
    InferenceGateway, ProcessSpawner, RerankRequest, RerankResponse,
};
use node_engine::ExecutorExtensions;
use pantograph_embedded_runtime::{EmbeddedRuntime, EmbeddedRuntimeConfig};
use pantograph_workflow_service::WorkflowService;
use tokio::sync::RwLock;

use super::FfiPantographRuntime;
use crate::FfiError;

struct ShutdownBackend {
    fail_next_stop: AtomicBool,
    ready: AtomicBool,
    stop_calls: Arc<AtomicUsize>,
}

#[async_trait]
impl InferenceBackend for ShutdownBackend {
    fn name(&self) -> &'static str {
        "shutdown-fixture"
    }
    fn description(&self) -> &'static str {
        "bounded shutdown fixture"
    }
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }
    async fn start(
        &mut self,
        _: &BackendConfig,
        _: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        panic!("shutdown fixture must not start a backend")
    }
    async fn stop(&mut self) -> Result<(), BackendError> {
        self.stop_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_next_stop.swap(false, Ordering::SeqCst) {
            return Err(BackendError::Inference("fixture stop failure".to_string()));
        }
        self.ready.store(false, Ordering::SeqCst);
        Ok(())
    }
    fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }
    async fn health_check(&self) -> bool {
        self.is_ready()
    }
    fn base_url(&self) -> Option<String> {
        None
    }
    async fn chat_completion_stream(
        &self,
        _: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        panic!("shutdown fixture must not execute inference")
    }
    async fn embeddings(
        &self,
        _: Vec<String>,
        _: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        panic!("shutdown fixture must not execute embeddings")
    }
    async fn rerank(&self, _: RerankRequest) -> Result<RerankResponse, BackendError> {
        panic!("shutdown fixture must not execute reranking")
    }
}

fn runtime(
    fail_next_stop: bool,
) -> (
    FfiPantographRuntime,
    Arc<InferenceGateway>,
    Arc<AtomicUsize>,
    std::path::PathBuf,
) {
    let root = super::runtime_tests::create_temp_root("shutdown-fixture");
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(ShutdownBackend {
            fail_next_stop: AtomicBool::new(fail_next_stop),
            ready: AtomicBool::new(true),
            stop_calls: stop_calls.clone(),
        }),
        "shutdown-fixture",
    ));
    let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
    let app_data_dir = root.join("app-data");
    let embedded = EmbeddedRuntime::with_default_python_runtime(
        EmbeddedRuntimeConfig::new(app_data_dir.clone(), root.clone()),
        gateway.clone(),
        extensions.clone(),
        Arc::new(WorkflowService::new()),
        None,
    );
    (
        FfiPantographRuntime {
            runtime: Arc::new(embedded),
            app_data_dir,
            node_registry: Arc::new(node_engine::NodeRegistry::with_builtins()),
            extensions,
        },
        gateway,
        stop_calls,
        root,
    )
}

#[tokio::test]
async fn ffi_shutdown_reports_owner_failure_and_allows_retry() {
    let (runtime, gateway, stop_calls, root) = runtime(true);
    assert!(gateway.is_ready().await);
    let error = runtime
        .shutdown()
        .await
        .expect_err("failed owner stop must cross FFI");
    let FfiError::Other { message } = error else {
        panic!("expected standard FFI error envelope");
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&message).unwrap(),
        serde_json::json!({
            "code": "internal_error", "message": "Backend error: Inference error: fixture stop failure"
        })
    );
    assert_eq!(stop_calls.load(Ordering::SeqCst), 1);
    assert!(
        gateway.is_ready().await,
        "failed stop must preserve backend readiness"
    );
    runtime
        .shutdown()
        .await
        .expect("retry observes successful owner stop");
    assert_eq!(stop_calls.load(Ordering::SeqCst), 2);
    assert!(!gateway.is_ready().await);
    std::fs::remove_dir_all(root).expect("remove fixture files");
}

#[tokio::test]
async fn ffi_shutdown_success_and_repeated_success_are_explicit() {
    let (runtime, gateway, stop_calls, root) = runtime(false);
    runtime.shutdown().await.expect("successful shutdown");
    assert!(!gateway.is_ready().await);
    runtime.shutdown().await.expect("repeated shutdown");
    assert_eq!(stop_calls.load(Ordering::SeqCst), 2);
    std::fs::remove_dir_all(root).expect("remove fixture files");
}
