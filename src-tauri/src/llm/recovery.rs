//! Automatic recovery for crashed LLM servers
//!
//! Handles restart attempts with exponential backoff.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::agent::rag::SharedRagManager;
use crate::config::{AppConfig, EmbeddingMemoryMode};
use crate::constants::ports;
use crate::llm::health_monitor::ServerEvent;
use crate::llm::port_manager::{check_port_available, find_available_port};
use crate::llm::runtime_registry::run_runtime_transition_and_sync_runtime_registry;
use crate::llm::runtime_registry::stop_all_and_sync_runtime_registry;
use crate::llm::startup::{
    require_configured_embedding_startup_devices, validate_external_server_url,
};
use crate::llm::sync_rag_embedding_url_from_gateway;
use crate::llm::{list_devices, SharedAppConfig, SharedGateway, SharedRuntimeRegistry};
use crate::workflow::runtime_shutdown::invalidate_loaded_session_runtimes;
use pantograph_embedded_runtime::embedding_model_config::resolve_configured_embedding_model_path;
use pantograph_embedded_runtime::runtime_recovery::{
    build_recovery_attempt_plan, build_recovery_restart_plan, recovery_backoff,
    RecoveryAttemptPlan, RecoveryStrategy,
};

/// Recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Whether automatic recovery is enabled
    pub auto_recovery_enabled: bool,
    /// Maximum number of recovery attempts
    pub max_attempts: u32,
    /// Base backoff time in milliseconds
    pub backoff_base_ms: u64,
    /// Maximum backoff time in milliseconds
    pub backoff_max_ms: u64,
    /// Whether to try alternate ports on failure
    pub try_alternate_port: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            auto_recovery_enabled: true,
            max_attempts: 3,
            backoff_base_ms: 1000,
            backoff_max_ms: 30000,
            try_alternate_port: true,
        }
    }
}

/// Recovery error
#[derive(Debug, Clone, Serialize)]
pub struct RecoveryError {
    pub message: String,
    pub attempts: u32,
    pub strategy_used: RecoveryStrategy,
}

impl std::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Recovery failed after {} attempts using {:?}: {}",
            self.attempts, self.strategy_used, self.message
        )
    }
}

/// Recovery manager state
pub struct RecoveryManager {
    config: RecoveryConfig,
    recovering: Arc<AtomicBool>,
    attempt_count: Arc<AtomicU32>,
    last_error: Arc<Mutex<Option<String>>>,
    auto_recovery_task: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl RecoveryManager {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            recovering: Arc::new(AtomicBool::new(false)),
            attempt_count: Arc::new(AtomicU32::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            auto_recovery_task: std::sync::Mutex::new(None),
        }
    }

    /// Check if recovery is currently in progress
    pub fn is_recovering(&self) -> bool {
        self.recovering.load(Ordering::SeqCst)
    }

    /// Get current attempt count
    pub fn attempt_count(&self) -> u32 {
        self.attempt_count.load(Ordering::SeqCst)
    }

    /// Reset recovery state (call after successful manual start)
    pub fn reset(&self) {
        self.recovering.store(false, Ordering::SeqCst);
        self.attempt_count.store(0, Ordering::SeqCst);
    }

    pub fn start_auto_recovery(
        self: &Arc<Self>,
        app: AppHandle,
        gateway: SharedGateway,
        failure_reason: String,
    ) {
        if self.is_recovering() {
            return;
        }

        let recovery_manager = Arc::clone(self);
        let auto_recovery_task = tokio::spawn(async move {
            if let Err(error) = recovery_manager
                .recover(&app, &gateway, &failure_reason)
                .await
            {
                log::warn!("Automatic recovery failed: {}", error);
            }
        });

        if let Err(auto_recovery_task) = self.track_auto_recovery_task(auto_recovery_task) {
            auto_recovery_task.abort();
        }
    }

    pub fn stop_auto_recovery_task(&self) {
        let auto_recovery_task = match self.auto_recovery_task.lock() {
            Ok(mut task) => task.take(),
            Err(error) => {
                log::error!("Failed to acquire auto-recovery task handle: {error}");
                return;
            }
        };

        if let Some(auto_recovery_task) = auto_recovery_task {
            auto_recovery_task.abort();
            self.recovering.store(false, Ordering::SeqCst);
        }
    }

    fn track_auto_recovery_task(&self, handle: JoinHandle<()>) -> Result<(), JoinHandle<()>> {
        let mut task = match self.auto_recovery_task.lock() {
            Ok(task) => task,
            Err(error) => {
                log::error!("Failed to track auto-recovery task: {error}");
                return Err(handle);
            }
        };

        if task
            .as_ref()
            .is_some_and(|existing| !existing.is_finished())
        {
            log::debug!("Auto-recovery task already tracked");
            return Err(handle);
        }

        *task = Some(handle);
        Ok(())
    }

    /// Attempt to recover the server
    ///
    /// Returns the port the server is now running on if successful.
    pub async fn recover(
        &self,
        app: &AppHandle,
        gateway: &SharedGateway,
        failure_reason: &str,
    ) -> Result<u16, RecoveryError> {
        if !self.config.auto_recovery_enabled {
            return Err(RecoveryError {
                message: "Auto-recovery is disabled".to_string(),
                attempts: 0,
                strategy_used: RecoveryStrategy::Abandon,
            });
        }

        // Check if already recovering
        if self.recovering.swap(true, Ordering::SeqCst) {
            return Err(RecoveryError {
                message: "Recovery already in progress".to_string(),
                attempts: self.attempt_count.load(Ordering::SeqCst),
                strategy_used: RecoveryStrategy::Restart,
            });
        }

        log::info!("Starting recovery for: {}", failure_reason);
        *self.last_error.lock().await = Some(failure_reason.to_string());

        // Emit recovery started event
        let event = ServerEvent::RecoveryStarted;
        let _ = app.emit("server-health", &event);

        let mut last_error = failure_reason.to_string();
        let mut strategy = RecoveryStrategy::Restart;

        while self.attempt_count.load(Ordering::SeqCst) < self.config.max_attempts {
            let attempt = self.attempt_count.fetch_add(1, Ordering::SeqCst);

            // Calculate and apply backoff
            let backoff = recovery_backoff(
                self.config.backoff_base_ms,
                self.config.backoff_max_ms,
                attempt,
            );
            log::info!("Recovery attempt {} (waiting {:?})", attempt + 1, backoff);
            tokio::time::sleep(backoff).await;

            let default_port_available = check_port_available(ports::SERVER).available;
            let alternate_port = if self.config.try_alternate_port {
                find_available_port(ports::ALTERNATE_START, ports::ALTERNATE_RANGE)
            } else {
                None
            };
            let attempt_plan = build_recovery_attempt_plan(
                attempt,
                self.config.try_alternate_port,
                default_port_available,
                alternate_port,
            );

            strategy = attempt_plan
                .as_ref()
                .map(|plan| plan.strategy.clone())
                .unwrap_or(RecoveryStrategy::Restart);

            match self
                .try_recovery_strategy(app, gateway, attempt_plan.as_ref())
                .await
            {
                Ok(port) => {
                    log::info!("Recovery successful on port {}", port);

                    // Emit success event
                    let event = ServerEvent::RecoveryComplete {
                        success: true,
                        error: None,
                    };
                    let _ = app.emit("server-health", &event);

                    self.reset();
                    return Ok(port);
                }
                Err(e) => {
                    last_error = e.clone();
                    log::warn!("Recovery attempt {} failed: {}", attempt + 1, e);
                }
            }
        }

        // Max attempts reached
        log::error!(
            "Recovery failed after {} attempts: {}",
            self.config.max_attempts,
            last_error
        );

        // Emit failure event
        let event = ServerEvent::RecoveryComplete {
            success: false,
            error: Some(last_error.clone()),
        };
        let _ = app.emit("server-health", &event);

        self.recovering.store(false, Ordering::SeqCst);

        Err(RecoveryError {
            message: last_error,
            attempts: self.config.max_attempts,
            strategy_used: strategy,
        })
    }

    /// Try a specific recovery strategy
    async fn try_recovery_strategy(
        &self,
        app: &AppHandle,
        gateway: &SharedGateway,
        attempt_plan: Result<
            &RecoveryAttemptPlan,
            &pantograph_embedded_runtime::runtime_recovery::RecoveryAttemptPlanError,
        >,
    ) -> Result<u16, String> {
        let attempt_plan = attempt_plan.map_err(|error| error.to_string())?;

        match attempt_plan.strategy {
            RecoveryStrategy::Restart => {
                self.do_restart(app, gateway, attempt_plan.port_override)
                    .await
            }
            RecoveryStrategy::AlternatePort => {
                if let Some(alt_port) = attempt_plan.port_override {
                    log::info!(
                        "Using alternate port {} (default {} is blocked)",
                        alt_port,
                        ports::SERVER
                    );
                }
                self.do_restart(app, gateway, attempt_plan.port_override)
                    .await
            }
            RecoveryStrategy::CleanRestart => {
                stop_gateway_for_recovery(app, gateway).await?;
                if !attempt_plan.settle_delay.is_zero() {
                    tokio::time::sleep(attempt_plan.settle_delay).await;
                }
                self.do_restart(app, gateway, attempt_plan.port_override)
                    .await
            }
            RecoveryStrategy::Abandon => Err("Abandoning recovery".to_string()),
        }
    }

    /// Perform the actual restart
    async fn do_restart(
        &self,
        app: &AppHandle,
        gateway: &SharedGateway,
        port_override: Option<u16>,
    ) -> Result<u16, String> {
        let app_config = app
            .try_state::<SharedAppConfig>()
            .ok_or_else(|| "Application config not initialized".to_string())?;
        let app_config = app_config.read().await.clone();
        let restart_plan = build_recovery_restart_plan(
            gateway.restart_runtime_config().await,
            port_override,
            app_config.models.embedding_model_path.is_some(),
            app_config.embedding_memory_mode != EmbeddingMemoryMode::Sequential,
        )
        .map_err(|error| error.to_string())?;

        // Stop existing before starting a replacement. A failed stop leaves
        // the current runtime owned and prevents recovery from claiming a
        // replacement was started.
        stop_gateway_for_recovery(app, gateway).await?;

        if let Some(runtime_registry) = app.try_state::<SharedRuntimeRegistry>() {
            run_runtime_transition_and_sync_runtime_registry(
                gateway.as_ref(),
                runtime_registry.as_ref(),
                |_| async {
                    gateway
                        .start(&restart_plan.restart_config)
                        .await
                        .map_err(|error| error.to_string())?;

                    if restart_plan.restart_embedding {
                        restart_dedicated_embedding_runtime(app, gateway, &app_config).await?;
                    } else {
                        sync_rag_embedding_url(app, gateway).await;
                    }

                    Ok::<(), String>(())
                },
            )
            .await?;
        } else {
            gateway
                .start(&restart_plan.restart_config)
                .await
                .map_err(|error| error.to_string())?;

            if restart_plan.restart_embedding {
                restart_dedicated_embedding_runtime(app, gateway, &app_config).await?;
            } else {
                sync_rag_embedding_url(app, gateway).await;
            }
        }

        Ok(recovery_port_from_gateway(gateway).await)
    }

    /// Get the configuration
    pub fn config(&self) -> &RecoveryConfig {
        &self.config
    }

    /// Get last error
    pub async fn last_error(&self) -> Option<String> {
        self.last_error.lock().await.clone()
    }
}

async fn stop_gateway_for_recovery(app: &AppHandle, gateway: &SharedGateway) -> Result<(), String> {
    let runtime_registry = app
        .try_state::<SharedRuntimeRegistry>()
        .map(|state| state.inner().as_ref());
    stop_gateway_and_sync_runtime_registry(gateway, runtime_registry).await?;
    invalidate_loaded_session_runtimes(app);
    Ok(())
}

async fn stop_gateway_and_sync_runtime_registry(
    gateway: &SharedGateway,
    runtime_registry: Option<&pantograph_runtime_registry::RuntimeRegistry>,
) -> Result<(), String> {
    if let Some(runtime_registry) = runtime_registry {
        stop_all_and_sync_runtime_registry(gateway.as_ref(), runtime_registry)
            .await
            .map_err(|error| error.to_string())
    } else {
        gateway.stop_all().await.map_err(|error| error.to_string())
    }
}

async fn restart_dedicated_embedding_runtime(
    app: &AppHandle,
    gateway: &SharedGateway,
    app_config: &AppConfig,
) -> Result<(), String> {
    let Some(embedding_model_path) = app_config.models.embedding_model_path.as_deref() else {
        sync_rag_embedding_url(app, gateway).await;
        return Ok(());
    };

    let resolved_embedding_path = resolve_configured_embedding_model_path(embedding_model_path)?;
    let devices = require_configured_embedding_startup_devices(list_devices(app.clone()).await)?;

    gateway
        .start_embedding_server(
            &resolved_embedding_path.to_string_lossy(),
            app_config.embedding_memory_mode.clone(),
            &devices,
        )
        .await
        .map_err(|error| error.to_string())?;
    sync_rag_embedding_url(app, gateway).await;
    Ok(())
}

async fn sync_rag_embedding_url(app: &AppHandle, gateway: &SharedGateway) {
    let Some(rag_manager) = app.try_state::<SharedRagManager>() else {
        return;
    };

    sync_rag_embedding_url_from_gateway(gateway, &rag_manager).await;
}

async fn recovery_port_from_gateway(gateway: &SharedGateway) -> u16 {
    gateway
        .base_url()
        .await
        .as_deref()
        .map(port_from_base_url)
        .unwrap_or(ports::SERVER)
}

fn port_from_base_url(base_url: &str) -> u16 {
    validate_external_server_url(base_url)
        .ok()
        .and_then(|normalized| reqwest::Url::parse(&normalized).ok())
        .and_then(|url| url.port_or_known_default())
        .unwrap_or(ports::SERVER)
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new(RecoveryConfig::default())
    }
}

/// Shared recovery manager type for Tauri state
pub type SharedRecoveryManager = Arc<RecoveryManager>;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_util::{stream, Stream};
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        InferenceBackend,
    };
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::{ImageGenerationRequest, ImageGenerationResult, RerankRequest, RerankResponse};
    use tokio::sync::mpsc;

    use super::{port_from_base_url, stop_gateway_and_sync_runtime_registry};
    use crate::constants::ports;

    struct MockProcessSpawner;

    #[async_trait]
    impl ProcessSpawner for MockProcessSpawner {
        async fn spawn_sidecar(
            &self,
            _sidecar_name: &str,
            _args: &[&str],
        ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
            Err("spawn should not be called in recovery stop tests".to_string())
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }
    }

    struct FailingStopBackend {
        ready: bool,
    }

    #[async_trait]
    impl InferenceBackend for FailingStopBackend {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn description(&self) -> &'static str {
            "Backend that refuses to stop"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            self.ready = true;
            Ok(BackendStartOutcome::default())
        }

        async fn stop(&mut self) -> Result<(), BackendError> {
            Err(BackendError::Inference("mock stop failure".to_string()))
        }

        fn is_ready(&self) -> bool {
            self.ready
        }

        async fn health_check(&self) -> bool {
            self.ready
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
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }

        async fn generate_image(
            &self,
            _request: ImageGenerationRequest,
        ) -> Result<ImageGenerationResult, BackendError> {
            Err(BackendError::Inference(
                "image generation not supported in recovery stop tests".to_string(),
            ))
        }
    }

    #[test]
    fn port_from_base_url_uses_known_default_when_port_missing() {
        assert_eq!(port_from_base_url("http://127.0.0.1:8080"), 8080);
        assert_eq!(port_from_base_url("https://example.test"), 443);
        assert_eq!(port_from_base_url("not-a-url"), ports::SERVER);
    }

    #[tokio::test]
    async fn recovery_stop_gate_propagates_failure_without_releasing_no_model_runtime() {
        let gateway = Arc::new(crate::llm::gateway::InferenceGateway::with_test_backend(
            Box::new(FailingStopBackend { ready: false }),
            "PyTorch",
            Arc::new(MockProcessSpawner),
        ));
        gateway.init().await;
        gateway
            .start(&BackendConfig::default())
            .await
            .expect("gateway should start");

        let error = stop_gateway_and_sync_runtime_registry(&gateway, None)
            .await
            .expect_err("recovery must stop before attempting a restart");
        assert!(error.contains("mock stop failure"));
        assert!(gateway.is_ready().await);
        assert!(gateway.runtime_lifecycle_snapshot().await.active);
        assert_eq!(gateway.current_backend_name().await, "PyTorch");
    }
}
