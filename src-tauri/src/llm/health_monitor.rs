//! Health monitoring for LLM servers
//!
//! Background monitoring that detects server crashes and emits Tauri events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::llm::recovery::SharedRecoveryManager;
use crate::llm::runtime_registry::sync_runtime_registry_from_gateway;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};
use pantograph_embedded_runtime::runtime_health::{
    RuntimeHealthAssessment, RuntimeHealthProbe, RuntimeHealthState, assess_runtime_health_probe,
};

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
    RecoveryComplete {
        success: bool,
        error: Option<String>,
    },
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
    embedding_consecutive_failures: Arc<RwLock<u32>>,
    monitor_task: std::sync::Mutex<Option<JoinHandle<()>>>,
}

impl HealthMonitor {
    pub fn new(config: HealthMonitorConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            last_result: Arc::new(RwLock::new(None)),
            consecutive_failures: Arc::new(RwLock::new(0)),
            embedding_consecutive_failures: Arc::new(RwLock::new(0)),
            monitor_task: std::sync::Mutex::new(None),
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
        let embedding_consecutive_failures = self.embedding_consecutive_failures.clone();
        let config = self.config.clone();

        log::info!(
            "Starting health monitor with {}s interval",
            config.check_interval.as_secs()
        );

        let monitor_task = tokio::spawn(async move {
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

                let active_base_url = gateway.base_url().await;
                let embedding_base_url = gateway.dedicated_embedding_base_url().await;

                if active_base_url.is_none() && embedding_base_url.is_none() {
                    *consecutive_failures.write().await = 0;
                    *embedding_consecutive_failures.write().await = 0;
                    *last_result.write().await = None;
                    gateway.set_runtime_health_assessments(None, None).await;
                    sync_runtime_registry(&app, &gateway).await;
                    previous_healthy = true;
                    tokio::time::sleep(config.check_interval).await;
                    continue;
                }

                let active_assessment = if let Some(base_url) = active_base_url.as_deref() {
                    Some(
                        assess_runtime_health_url(
                            &format!("{}/health", base_url),
                            config.request_timeout,
                            config.failure_threshold,
                            &consecutive_failures,
                        )
                        .await,
                    )
                } else {
                    *consecutive_failures.write().await = 0;
                    None
                };

                let embedding_assessment = if let Some(base_url) = embedding_base_url.as_deref() {
                    Some(
                        assess_runtime_health_url(
                            &format!("{}/health", base_url),
                            config.request_timeout,
                            config.failure_threshold,
                            &embedding_consecutive_failures,
                        )
                        .await,
                    )
                } else {
                    *embedding_consecutive_failures.write().await = 0;
                    None
                };

                let Some(assessment) = active_assessment.clone() else {
                    *last_result.write().await = None;
                    gateway
                        .set_runtime_health_assessments(None, embedding_assessment.clone())
                        .await;
                    sync_runtime_registry(&app, &gateway).await;
                    previous_healthy = true;
                    tokio::time::sleep(config.check_interval).await;
                    continue;
                };

                let result = health_check_result_from_assessment(assessment.clone(), Utc::now());

                // Detect state changes
                let current_healthy = result.healthy;
                let state_changed = current_healthy != previous_healthy;

                // Emit event
                if config.emit_every_check || state_changed {
                    if state_changed && !current_healthy {
                        // Server just became unhealthy
                        let failure_reason = result
                            .error
                            .clone()
                            .unwrap_or_else(|| "Unknown error".to_string());
                        let event = ServerEvent::ServerCrashed {
                            error: failure_reason.clone(),
                        };
                        if let Err(e) = app.emit("server-health", &event) {
                            log::warn!("Failed to emit server crashed event: {}", e);
                        }

                        maybe_start_auto_recovery(&app, &gateway, &failure_reason).await;
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
                gateway
                    .set_runtime_health_assessments(
                        Some(assessment.clone()),
                        embedding_assessment.clone(),
                    )
                    .await;
                sync_runtime_registry(&app, &gateway).await;
                previous_healthy = current_healthy;

                tokio::time::sleep(config.check_interval).await;
            }

            log::info!("Health monitor stopped");
        });

        match self.monitor_task.lock() {
            Ok(mut task) => {
                if let Some(previous_task) = task.replace(monitor_task) {
                    previous_task.abort();
                }
            }
            Err(error) => {
                log::error!("Failed to track health monitor task: {error}");
                self.running.store(false, Ordering::SeqCst);
                monitor_task.abort();
            }
        }
    }

    /// Stop health monitoring
    pub fn stop(&self) {
        if self.running.swap(false, Ordering::SeqCst) {
            log::info!("Stopping health monitor");
        }
        self.abort_monitor_task();
    }

    /// Check if the monitor is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the last health check result
    pub async fn last_result(&self) -> Option<HealthCheckResult> {
        self.last_result.read().await.clone()
    }

    #[cfg(test)]
    pub(crate) async fn set_test_last_result(&self, result: Option<HealthCheckResult>) {
        *self.last_result.write().await = result;
    }

    /// Get current consecutive failure count
    pub async fn consecutive_failures(&self) -> u32 {
        *self.consecutive_failures.read().await
    }

    /// Manually trigger a health check (outside of the regular interval)
    pub async fn check_now(&self, app: &AppHandle) -> Option<HealthCheckResult> {
        let gateway = app.try_state::<SharedGateway>()?;
        let active_base_url = gateway.base_url().await;
        let embedding_base_url = gateway.dedicated_embedding_base_url().await;

        let active_assessment = if let Some(base_url) = active_base_url.as_deref() {
            Some(
                assess_runtime_health_url(
                    &format!("{}/health", base_url),
                    self.config.request_timeout,
                    self.config.failure_threshold,
                    &self.consecutive_failures,
                )
                .await,
            )
        } else {
            *self.consecutive_failures.write().await = 0;
            None
        };

        let embedding_assessment = if let Some(base_url) = embedding_base_url.as_deref() {
            Some(
                assess_runtime_health_url(
                    &format!("{}/health", base_url),
                    self.config.request_timeout,
                    self.config.failure_threshold,
                    &self.embedding_consecutive_failures,
                )
                .await,
            )
        } else {
            *self.embedding_consecutive_failures.write().await = 0;
            None
        };

        let Some(assessment) = active_assessment else {
            *self.last_result.write().await = None;
            gateway
                .inner()
                .set_runtime_health_assessments(None, embedding_assessment.clone())
                .await;
            sync_runtime_registry(app, gateway.inner()).await;
            return None;
        };

        let result = health_check_result_from_assessment(assessment.clone(), Utc::now());

        // Update stored result
        *self.last_result.write().await = Some(result.clone());
        gateway
            .inner()
            .set_runtime_health_assessments(Some(assessment.clone()), embedding_assessment.clone())
            .await;
        sync_runtime_registry(app, gateway.inner()).await;

        Some(result)
    }

    fn abort_monitor_task(&self) {
        let monitor_task = match self.monitor_task.lock() {
            Ok(mut task) => task.take(),
            Err(error) => {
                log::error!("Failed to acquire health monitor task handle: {error}");
                return;
            }
        };

        if let Some(monitor_task) = monitor_task {
            monitor_task.abort();
        }
    }
}

async fn sync_runtime_registry(app: &AppHandle, gateway: &SharedGateway) {
    let Some(runtime_registry) = app.try_state::<SharedRuntimeRegistry>() else {
        return;
    };

    sync_runtime_registry_from_gateway(gateway.as_ref(), runtime_registry.as_ref()).await;
}

async fn maybe_start_auto_recovery(app: &AppHandle, gateway: &SharedGateway, failure_reason: &str) {
    let Some(recovery_manager) = app.try_state::<SharedRecoveryManager>() else {
        return;
    };
    let recovery_manager = (*recovery_manager).clone();

    if recovery_manager.is_recovering() {
        return;
    }

    let app = app.clone();
    let gateway = gateway.clone();
    let failure_reason = failure_reason.to_string();
    tokio::spawn(async move {
        if let Err(error) = recovery_manager
            .recover(&app, &gateway, &failure_reason)
            .await
        {
            log::warn!("Automatic recovery failed: {}", error);
        }
    });
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(HealthMonitorConfig::default())
    }
}

/// Shared health monitor type for Tauri state
pub type SharedHealthMonitor = Arc<HealthMonitor>;

async fn assess_runtime_health_url(
    url: &str,
    request_timeout: Duration,
    failure_threshold: u32,
    failures: &Arc<RwLock<u32>>,
) -> RuntimeHealthAssessment {
    let start = std::time::Instant::now();
    let client = reqwest::Client::builder()
        .timeout(request_timeout)
        .build()
        .unwrap_or_default();

    let check_result = client.get(url).send().await;
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let probe = probe_from_http_result(check_result, elapsed_ms);

    let mut failures = failures.write().await;
    let assessment = assess_runtime_health_probe(probe, *failures, failure_threshold);
    *failures = assessment.consecutive_failures;
    assessment
}

fn probe_from_http_result(
    result: Result<reqwest::Response, reqwest::Error>,
    elapsed_ms: u64,
) -> RuntimeHealthProbe {
    match result {
        Ok(response) if response.status().is_success() => RuntimeHealthProbe::Healthy {
            response_time_ms: elapsed_ms,
        },
        Ok(response) => RuntimeHealthProbe::Failed {
            reason: format!("HTTP {}", response.status()),
            response_time_ms: Some(elapsed_ms),
        },
        Err(error) => RuntimeHealthProbe::Failed {
            reason: health_failure_reason(&error),
            response_time_ms: None,
        },
    }
}

fn health_failure_reason(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "Request timeout".to_string()
    } else if error.is_connect() {
        "Connection refused".to_string()
    } else {
        error.to_string()
    }
}

fn health_check_result_from_assessment(
    assessment: RuntimeHealthAssessment,
    timestamp: DateTime<Utc>,
) -> HealthCheckResult {
    let status = match assessment.state {
        RuntimeHealthState::Healthy => HealthStatus::Healthy,
        RuntimeHealthState::Degraded { reason } => HealthStatus::Degraded { reason },
        RuntimeHealthState::Unhealthy { reason } => HealthStatus::Unhealthy { reason },
    };

    HealthCheckResult {
        healthy: assessment.healthy,
        status,
        response_time_ms: assessment.response_time_ms,
        error: assessment.error,
        timestamp,
        consecutive_failures: assessment.consecutive_failures,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use pantograph_embedded_runtime::runtime_health::{
        RuntimeHealthAssessment, RuntimeHealthState,
    };

    use super::{HealthStatus, health_check_result_from_assessment};

    #[test]
    fn health_check_result_maps_degraded_backend_state() {
        let result = health_check_result_from_assessment(
            RuntimeHealthAssessment {
                healthy: true,
                state: RuntimeHealthState::Degraded {
                    reason: "HTTP 503".to_string(),
                },
                response_time_ms: Some(55),
                error: Some("HTTP 503".to_string()),
                consecutive_failures: 2,
            },
            Utc::now(),
        );

        assert!(result.healthy);
        assert_eq!(
            result.status,
            HealthStatus::Degraded {
                reason: "HTTP 503".to_string(),
            }
        );
        assert_eq!(result.response_time_ms, Some(55));
        assert_eq!(result.error.as_deref(), Some("HTTP 503"));
        assert_eq!(result.consecutive_failures, 2);
    }

    #[test]
    fn health_check_result_maps_unhealthy_backend_state() {
        let result = health_check_result_from_assessment(
            RuntimeHealthAssessment {
                healthy: false,
                state: RuntimeHealthState::Unhealthy {
                    reason: "Connection refused".to_string(),
                },
                response_time_ms: None,
                error: Some("Connection refused".to_string()),
                consecutive_failures: 3,
            },
            Utc::now(),
        );

        assert!(!result.healthy);
        assert_eq!(
            result.status,
            HealthStatus::Unhealthy {
                reason: "Connection refused".to_string(),
            }
        );
        assert_eq!(result.response_time_ms, None);
        assert_eq!(result.error.as_deref(), Some("Connection refused"));
        assert_eq!(result.consecutive_failures, 3);
    }
}
