//! Health monitoring and recovery commands
//!
//! Commands for health checking and recovery management.

use tauri::{command, AppHandle, Manager, State};

use super::shared::sync_runtime_registry_from_gateway;
use crate::llm::health_monitor::{HealthCheckResult, SharedHealthMonitor};
use crate::llm::recovery::{RecoveryConfig, RecoveryError, SharedRecoveryManager};
use crate::llm::{SharedGateway, SharedRuntimeRegistry};

fn shared_health_monitor(app: &AppHandle) -> Result<SharedHealthMonitor, String> {
    app.try_state::<SharedHealthMonitor>()
        .map(|state| (*state).clone())
        .ok_or_else(|| "Health monitor not initialized".to_string())
}

fn shared_recovery_manager(app: &AppHandle) -> Result<SharedRecoveryManager, String> {
    app.try_state::<SharedRecoveryManager>()
        .map(|state| (*state).clone())
        .ok_or_else(|| "Recovery manager not initialized".to_string())
}

/// Start health monitoring
#[command]
pub async fn start_health_monitor(app: AppHandle) -> Result<(), String> {
    let monitor = shared_health_monitor(&app)?;

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
    let Some(monitor) = app.try_state::<SharedHealthMonitor>() else {
        return Ok(None);
    };

    Ok(monitor.last_result().await)
}

/// Trigger an immediate health check
#[command]
pub async fn check_health_now(app: AppHandle) -> Result<Option<HealthCheckResult>, String> {
    let Some(monitor) = app.try_state::<SharedHealthMonitor>() else {
        return Ok(None);
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
    let Some(manager) = app.try_state::<SharedRecoveryManager>() else {
        return Ok(RecoveryConfig::default());
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
    runtime_registry: State<'_, SharedRuntimeRegistry>,
) -> Result<u16, String> {
    let manager = shared_recovery_manager(&app)?;

    let result = manager
        .recover(&app, &gateway, "Manual recovery triggered")
        .await
        .map_err(|e: RecoveryError| e.to_string());

    sync_runtime_registry_from_gateway(gateway.inner(), runtime_registry.inner()).await;
    result
}

/// Reset recovery state (after manual intervention)
#[command]
pub fn reset_recovery_state(app: AppHandle) -> Result<(), String> {
    let manager = shared_recovery_manager(&app)?;

    manager.reset();
    Ok(())
}
