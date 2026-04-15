//! Automatic recovery for crashed LLM servers
//!
//! Handles restart attempts with exponential backoff.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use crate::agent::rag::SharedRagManager;
use crate::config::{AppConfig, EmbeddingMemoryMode};
use crate::constants::ports;
use crate::llm::health_monitor::ServerEvent;
use crate::llm::port_manager::{check_port_available, find_available_port};
use crate::llm::runtime_registry::stop_all_and_sync_runtime_registry;
use crate::llm::runtime_registry::sync_runtime_registry_from_gateway;
use crate::llm::startup::validate_external_server_url;
use crate::llm::{list_devices, SharedAppConfig, SharedGateway, SharedRuntimeRegistry};
use pantograph_embedded_runtime::embedding_workflow::resolve_embedding_model_path;
use pantograph_embedded_runtime::runtime_recovery::{
    build_recovery_restart_plan, recovery_backoff, recovery_strategy_for_attempt, RecoveryStrategy,
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
}

impl RecoveryManager {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            recovering: Arc::new(AtomicBool::new(false)),
            attempt_count: Arc::new(AtomicU32::new(0)),
            last_error: Arc::new(Mutex::new(None)),
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
            let backoff =
                recovery_backoff(self.config.backoff_base_ms, self.config.backoff_max_ms, attempt);
            log::info!("Recovery attempt {} (waiting {:?})", attempt + 1, backoff);
            tokio::time::sleep(backoff).await;

            // Determine strategy based on attempt and config
            strategy = recovery_strategy_for_attempt(attempt, self.config.try_alternate_port);

            match self.try_recovery_strategy(app, gateway, &strategy).await {
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
        strategy: &RecoveryStrategy,
    ) -> Result<u16, String> {
        match strategy {
            RecoveryStrategy::Restart => {
                // Just try to restart with current config
                self.do_restart(app, gateway, None).await
            }
            RecoveryStrategy::AlternatePort => {
                // Check if default port is blocked
                let port_status = check_port_available(ports::SERVER);
                if !port_status.available {
                    // Find alternate port
                    let alt_port =
                        find_available_port(ports::ALTERNATE_START, ports::ALTERNATE_RANGE)
                            .ok_or_else(|| "No alternate ports available".to_string())?;

                    log::info!(
                        "Using alternate port {} (default {} is blocked)",
                        alt_port,
                        ports::SERVER
                    );
                    self.do_restart(app, gateway, Some(alt_port)).await
                } else {
                    self.do_restart(app, gateway, None).await
                }
            }
            RecoveryStrategy::CleanRestart => {
                // First stop any existing server
                stop_gateway_for_recovery(app, gateway).await;
                tokio::time::sleep(Duration::from_millis(500)).await;

                // Then restart
                self.do_restart(app, gateway, None).await
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
            .map(|config| config.clone())
            .ok_or_else(|| "Application config not initialized".to_string())?;
        let app_config = app_config.read().await.clone();
        let restart_plan = build_recovery_restart_plan(
            gateway.restart_runtime_config().await,
            port_override,
            app_config.models.embedding_model_path.is_some(),
            app_config.embedding_memory_mode != EmbeddingMemoryMode::Sequential,
        )
        .map_err(|error| error.to_string())?;

        // Stop existing
        stop_gateway_for_recovery(app, gateway).await;

        gateway
            .start(&restart_plan.restart_config)
            .await
            .map_err(|error| error.to_string())?;

        if restart_plan.restart_embedding {
            restart_dedicated_embedding_runtime(app, gateway, &app_config).await?;
        } else {
            sync_rag_embedding_url(app, gateway).await;
        }

        sync_runtime_registry_after_recovery_restart(app, gateway).await;

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

async fn stop_gateway_for_recovery(app: &AppHandle, gateway: &SharedGateway) {
    let Some(runtime_registry) = app
        .try_state::<SharedRuntimeRegistry>()
        .map(|runtime_registry| runtime_registry.clone())
    else {
        gateway.stop_all().await;
        return;
    };

    stop_all_and_sync_runtime_registry(gateway.as_ref(), runtime_registry.as_ref()).await;
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

    let resolved_embedding_path = resolve_embedding_model_path(embedding_model_path)?;
    let devices = list_devices(app.clone()).await.unwrap_or_default();

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
    let Some(rag_manager) = app
        .try_state::<SharedRagManager>()
        .map(|rag_manager| rag_manager.clone())
    else {
        return;
    };

    let mut rag_guard = rag_manager.write().await;
    if let Some(url) = gateway.embedding_url().await {
        rag_guard.set_embedding_url(url);
    } else {
        rag_guard.clear_embedding_url();
    }
}

async fn sync_runtime_registry_after_recovery_restart(app: &AppHandle, gateway: &SharedGateway) {
    let Some(runtime_registry) = app
        .try_state::<SharedRuntimeRegistry>()
        .map(|runtime_registry| runtime_registry.clone())
    else {
        return;
    };

    sync_runtime_registry_from_gateway(gateway.as_ref(), runtime_registry.as_ref()).await;
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

#[cfg(test)]
mod tests {
    use super::port_from_base_url;
    use crate::constants::ports;

    #[test]
    fn port_from_base_url_uses_known_default_when_port_missing() {
        assert_eq!(port_from_base_url("http://127.0.0.1:8080"), 8080);
        assert_eq!(port_from_base_url("https://example.test"), 443);
        assert_eq!(port_from_base_url("not-a-url"), ports::SERVER);
    }
}

/// Shared recovery manager type for Tauri state
pub type SharedRecoveryManager = Arc<RecoveryManager>;
