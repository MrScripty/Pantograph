//! Health monitoring for LLM servers
//!
//! Background monitoring that detects server crashes and emits Tauri events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock;

use crate::llm::SharedGateway;

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Whether the server is healthy
    pub healthy: bool,
    /// Current health status
    pub status: HealthStatus,
    /// Response time in milliseconds (if successful)
    pub response_time_ms: Option<u64>,
    /// Error message (if unhealthy)
    pub error: Option<String>,
    /// Timestamp of the check
    pub timestamp: DateTime<Utc>,
    /// Consecutive failure count
    pub consecutive_failures: u32,
}

/// Health status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Server is responding normally
    Healthy,
    /// Server is responding but with issues
    Degraded { reason: String },
    /// Server is not responding
    Unhealthy { reason: String },
    /// Health status unknown (monitoring not started)
    Unknown,
}

/// Server event types emitted to frontend
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    /// Regular health update
    HealthUpdate { result: HealthCheckResult },
    /// Server crashed/became unresponsive
    ServerCrashed { error: String },
    /// Recovery attempt started
    RecoveryStarted,
    /// Recovery completed
    RecoveryComplete { success: bool, error: Option<String> },
}

/// Health monitor configuration
#[derive(Debug, Clone)]
pub struct HealthMonitorConfig {
    /// How often to check health
    pub check_interval: Duration,
    /// Number of consecutive failures before declaring unhealthy
    pub failure_threshold: u32,
    /// HTTP request timeout
    pub request_timeout: Duration,
    /// Whether to emit events for each check (vs only on state changes)
    pub emit_every_check: bool,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            failure_threshold: 3,
            request_timeout: Duration::from_secs(5),
            emit_every_check: false,
        }
    }
}

/// Health monitor state
pub struct HealthMonitor {
    config: HealthMonitorConfig,
    running: Arc<AtomicBool>,
    last_result: Arc<RwLock<Option<HealthCheckResult>>>,
    consecutive_failures: Arc<RwLock<u32>>,
}

impl HealthMonitor {
    pub fn new(config: HealthMonitorConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            last_result: Arc::new(RwLock::new(None)),
            consecutive_failures: Arc::new(RwLock::new(0)),
        }
    }

    /// Start background health monitoring
    pub fn start(&self, app: AppHandle) {
        if self.running.swap(true, Ordering::SeqCst) {
            log::warn!("Health monitor already running");
            return;
        }

        let running = self.running.clone();
        let last_result = self.last_result.clone();
        let consecutive_failures = self.consecutive_failures.clone();
        let config = self.config.clone();

        log::info!(
            "Starting health monitor with {}s interval",
            config.check_interval.as_secs()
        );

        tokio::spawn(async move {
            let mut previous_healthy = true;

            while running.load(Ordering::SeqCst) {
                // Get gateway from app state
                let gateway = match app.try_state::<SharedGateway>() {
                    Some(g) => g.clone(),
                    None => {
                        tokio::time::sleep(config.check_interval).await;
                        continue;
                    }
                };

                // Check if we have an active server
                let base_url = gateway.base_url().await;
                if base_url.is_none() {
                    // No server running, reset state
                    *consecutive_failures.write().await = 0;
                    *last_result.write().await = None;
                    previous_healthy = true;
                    tokio::time::sleep(config.check_interval).await;
                    continue;
                }

                let url = format!("{}/health", base_url.unwrap());
                let start = std::time::Instant::now();

                // Perform health check
                let client = reqwest::Client::builder()
                    .timeout(config.request_timeout)
                    .build()
                    .unwrap_or_default();

                let check_result = client.get(&url).send().await;
                let elapsed_ms = start.elapsed().as_millis() as u64;

                let result = match check_result {
                    Ok(resp) if resp.status().is_success() => {
                        // Reset failure count on success
                        *consecutive_failures.write().await = 0;

                        HealthCheckResult {
                            healthy: true,
                            status: HealthStatus::Healthy,
                            response_time_ms: Some(elapsed_ms),
                            error: None,
                            timestamp: Utc::now(),
                            consecutive_failures: 0,
                        }
                    }
                    Ok(resp) => {
                        // Non-success status code
                        let mut failures = consecutive_failures.write().await;
                        *failures += 1;
                        let fail_count = *failures;

                        let reason = format!("HTTP {}", resp.status());
                        let status = if fail_count >= config.failure_threshold {
                            HealthStatus::Unhealthy { reason: reason.clone() }
                        } else {
                            HealthStatus::Degraded { reason: reason.clone() }
                        };

                        HealthCheckResult {
                            healthy: fail_count < config.failure_threshold,
                            status,
                            response_time_ms: Some(elapsed_ms),
                            error: Some(reason),
                            timestamp: Utc::now(),
                            consecutive_failures: fail_count,
                        }
                    }
                    Err(e) => {
                        // Request failed
                        let mut failures = consecutive_failures.write().await;
                        *failures += 1;
                        let fail_count = *failures;

                        let reason = if e.is_timeout() {
                            "Request timeout".to_string()
                        } else if e.is_connect() {
                            "Connection refused".to_string()
                        } else {
                            e.to_string()
                        };

                        let status = if fail_count >= config.failure_threshold {
                            HealthStatus::Unhealthy { reason: reason.clone() }
                        } else {
                            HealthStatus::Degraded { reason: reason.clone() }
                        };

                        HealthCheckResult {
                            healthy: fail_count < config.failure_threshold,
                            status,
                            response_time_ms: None,
                            error: Some(reason),
                            timestamp: Utc::now(),
                            consecutive_failures: fail_count,
                        }
                    }
                };

                // Detect state changes
                let current_healthy = result.healthy;
                let state_changed = current_healthy != previous_healthy;

                // Emit event
                if config.emit_every_check || state_changed {
                    if state_changed && !current_healthy {
                        // Server just became unhealthy
                        let event = ServerEvent::ServerCrashed {
                            error: result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                        };
                        if let Err(e) = app.emit("server-health", &event) {
                            log::warn!("Failed to emit server crashed event: {}", e);
                        }
                    }

                    let event = ServerEvent::HealthUpdate {
                        result: result.clone(),
                    };
                    if let Err(e) = app.emit("server-health", &event) {
                        log::warn!("Failed to emit health event: {}", e);
                    }
                }

                // Update state
                *last_result.write().await = Some(result);
                previous_healthy = current_healthy;

                tokio::time::sleep(config.check_interval).await;
            }

            log::info!("Health monitor stopped");
        });
    }

    /// Stop health monitoring
    pub fn stop(&self) {
        if self.running.swap(false, Ordering::SeqCst) {
            log::info!("Stopping health monitor");
        }
    }

    /// Check if the monitor is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the last health check result
    pub async fn last_result(&self) -> Option<HealthCheckResult> {
        self.last_result.read().await.clone()
    }

    /// Get current consecutive failure count
    pub async fn consecutive_failures(&self) -> u32 {
        *self.consecutive_failures.read().await
    }

    /// Manually trigger a health check (outside of the regular interval)
    pub async fn check_now(&self, app: &AppHandle) -> Option<HealthCheckResult> {
        let gateway = app.try_state::<SharedGateway>()?;
        let base_url = gateway.base_url().await?;
        let url = format!("{}/health", base_url);

        let start = std::time::Instant::now();
        let client = reqwest::Client::builder()
            .timeout(self.config.request_timeout)
            .build()
            .ok()?;

        let resp = client.get(&url).send().await;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let result = match resp {
            Ok(r) if r.status().is_success() => HealthCheckResult {
                healthy: true,
                status: HealthStatus::Healthy,
                response_time_ms: Some(elapsed_ms),
                error: None,
                timestamp: Utc::now(),
                consecutive_failures: 0,
            },
            Ok(r) => HealthCheckResult {
                healthy: false,
                status: HealthStatus::Unhealthy {
                    reason: format!("HTTP {}", r.status()),
                },
                response_time_ms: Some(elapsed_ms),
                error: Some(format!("HTTP {}", r.status())),
                timestamp: Utc::now(),
                consecutive_failures: 1,
            },
            Err(e) => HealthCheckResult {
                healthy: false,
                status: HealthStatus::Unhealthy {
                    reason: e.to_string(),
                },
                response_time_ms: None,
                error: Some(e.to_string()),
                timestamp: Utc::now(),
                consecutive_failures: 1,
            },
        };

        // Update stored result
        *self.last_result.write().await = Some(result.clone());

        Some(result)
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(HealthMonitorConfig::default())
    }
}

/// Shared health monitor type for Tauri state
pub type SharedHealthMonitor = Arc<HealthMonitor>;
