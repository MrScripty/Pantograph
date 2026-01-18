//! Health monitoring and recovery commands
//!
//! Commands for health checking and recovery management.

use std::sync::Arc;
use tauri::{command, AppHandle, Manager, State};

use crate::llm::health_monitor::{
    HealthCheckResult, HealthMonitor, HealthMonitorConfig, SharedHealthMonitor,
};
use crate::llm::recovery::{RecoveryConfig, RecoveryError, RecoveryManager, SharedRecoveryManager};
use crate::llm::SharedGateway;

/// Start health monitoring
#[command]
pub async fn start_health_monitor(app: AppHandle) -> Result<(), String> {
    // Get or create health monitor
    let monitor: Arc<HealthMonitor> = match app.try_state::<SharedHealthMonitor>() {
        Some(m) => (*m).clone(),
        None => {
            let monitor = Arc::new(HealthMonitor::new(HealthMonitorConfig::default()));
            app.manage(monitor.clone());
            monitor
        }
    };

    if monitor.is_running() {
        return Err("Health monitor already running".to_string());
    }

    monitor.start(app);
    Ok(())
}

/// Stop health monitoring
#[command]
pub async fn stop_health_monitor(app: AppHandle) -> Result<(), String> {
    let monitor = app
        .try_state::<SharedHealthMonitor>()
        .ok_or("Health monitor not initialized")?;

    monitor.stop();
    Ok(())
}

/// Get the last health check result
#[command]
pub async fn get_health_status(app: AppHandle) -> Result<Option<HealthCheckResult>, String> {
    let monitor = match app.try_state::<SharedHealthMonitor>() {
        Some(m) => m,
        None => return Ok(None),
    };

    Ok(monitor.last_result().await)
}

/// Trigger an immediate health check
#[command]
pub async fn check_health_now(app: AppHandle) -> Result<Option<HealthCheckResult>, String> {
    let monitor = match app.try_state::<SharedHealthMonitor>() {
        Some(m) => m,
        None => {
            // Create a temporary monitor for one-shot check
            let temp = HealthMonitor::new(HealthMonitorConfig::default());
            return Ok(temp.check_now(&app).await);
        }
    };

    Ok(monitor.check_now(&app).await)
}

/// Check if health monitoring is active
#[command]
pub fn is_health_monitor_running(app: AppHandle) -> bool {
    app.try_state::<SharedHealthMonitor>()
        .map(|m| m.is_running())
        .unwrap_or(false)
}

/// Get recovery configuration
#[command]
pub async fn get_recovery_config(app: AppHandle) -> Result<RecoveryConfig, String> {
    let manager = match app.try_state::<SharedRecoveryManager>() {
        Some(m) => m,
        None => return Ok(RecoveryConfig::default()),
    };

    Ok(manager.config().clone())
}

/// Check if recovery is in progress
#[command]
pub fn is_recovery_in_progress(app: AppHandle) -> bool {
    app.try_state::<SharedRecoveryManager>()
        .map(|m| m.is_recovering())
        .unwrap_or(false)
}

/// Get recovery attempt count
#[command]
pub fn get_recovery_attempt_count(app: AppHandle) -> u32 {
    app.try_state::<SharedRecoveryManager>()
        .map(|m| m.attempt_count())
        .unwrap_or(0)
}

/// Trigger manual recovery
#[command]
pub async fn trigger_recovery(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
) -> Result<u16, String> {
    // Get or create recovery manager
    let manager: Arc<RecoveryManager> = match app.try_state::<SharedRecoveryManager>() {
        Some(m) => (*m).clone(),
        None => {
            let manager = Arc::new(RecoveryManager::new(RecoveryConfig::default()));
            app.manage(manager.clone());
            manager
        }
    };

    manager
        .recover(&app, &gateway, "Manual recovery triggered")
        .await
        .map_err(|e: RecoveryError| e.to_string())
}

/// Reset recovery state (after manual intervention)
#[command]
pub fn reset_recovery_state(app: AppHandle) -> Result<(), String> {
    let manager = app
        .try_state::<SharedRecoveryManager>()
        .ok_or("Recovery manager not initialized")?;

    manager.reset();
    Ok(())
}
