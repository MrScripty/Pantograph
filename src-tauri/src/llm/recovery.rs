//! Automatic recovery for crashed LLM servers
//!
//! Handles restart attempts with exponential backoff.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::llm::health_monitor::ServerEvent;
use crate::llm::port_manager::{check_port_available, find_available_port};
use crate::llm::SharedGateway;
use crate::constants::ports;

/// Recovery strategy to use
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStrategy {
    /// Simply restart the server
    Restart,
    /// Try an alternate port if the original is in use
    AlternatePort,
    /// Clean restart (kill any existing, then start fresh)
    CleanRestart,
    /// Give up after max attempts
    Abandon,
}

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

    /// Calculate backoff delay for current attempt
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        let base = self.config.backoff_base_ms;
        let max = self.config.backoff_max_ms;

        // Exponential backoff: base * 2^attempt
        let delay_ms = base.saturating_mul(1u64 << attempt.min(10));
        let capped_ms = delay_ms.min(max);

        Duration::from_millis(capped_ms)
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
            let backoff = self.calculate_backoff(attempt);
            log::info!(
                "Recovery attempt {} (waiting {:?})",
                attempt + 1,
                backoff
            );
            tokio::time::sleep(backoff).await;

            // Determine strategy based on attempt and config
            strategy = if attempt == 0 {
                RecoveryStrategy::Restart
            } else if attempt == 1 && self.config.try_alternate_port {
                RecoveryStrategy::AlternatePort
            } else if attempt >= 2 {
                RecoveryStrategy::CleanRestart
            } else {
                RecoveryStrategy::Restart
            };

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
                    let alt_port = find_available_port(ports::ALTERNATE_START, ports::ALTERNATE_RANGE)
                        .ok_or_else(|| "No alternate ports available".to_string())?;

                    log::info!("Using alternate port {} (default {} is blocked)", alt_port, ports::SERVER);
                    self.do_restart(app, gateway, Some(alt_port)).await
                } else {
                    self.do_restart(app, gateway, None).await
                }
            }
            RecoveryStrategy::CleanRestart => {
                // First stop any existing server
                gateway.stop_all().await;
                tokio::time::sleep(Duration::from_millis(500)).await;

                // Then restart
                self.do_restart(app, gateway, None).await
            }
            RecoveryStrategy::Abandon => {
                Err("Abandoning recovery".to_string())
            }
        }
    }

    /// Perform the actual restart
    async fn do_restart(
        &self,
        app: &AppHandle,
        gateway: &SharedGateway,
        _port_override: Option<u16>,
    ) -> Result<u16, String> {
        // Get last known configuration from gateway
        // For now, we'll use a simplified restart that just starts inference
        // In a full implementation, we'd store the last config and restore it

        // Stop existing
        gateway.stop_all().await;

        // Start with stored config
        // Note: This requires the gateway to remember its last successful configuration
        // For now, this is a placeholder - the actual implementation would need
        // to store and restore the full server configuration

        Err("Restart not fully implemented - manual restart required. Use the UI to reconnect.".to_string())
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

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new(RecoveryConfig::default())
    }
}

/// Shared recovery manager type for Tauri state
pub type SharedRecoveryManager = Arc<RecoveryManager>;
