use crate::config::ServerModeInfo;
use crate::llm::commands::shared::synced_server_mode_info;
use crate::llm::health_monitor::{HealthCheckResult, SharedHealthMonitor};
use crate::llm::recovery::{RecoveryConfig, SharedRecoveryManager};
use crate::llm::runtime_registry::runtime_registry_snapshot as synced_runtime_registry_snapshot;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};
use crate::workflow::diagnostics::{
    DiagnosticsRuntimeSnapshot, DiagnosticsSchedulerSnapshot, WorkflowDiagnosticsProjection,
};
use pantograph_workflow_service::{WorkflowTraceRuntimeSelection, WorkflowTraceSnapshotResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeDebugTraceSelection {
    pub execution_id: Option<String>,
    #[serde(default)]
    pub matched_execution_ids: Vec<String>,
    pub ambiguous: bool,
}

impl From<WorkflowTraceRuntimeSelection> for RuntimeDebugTraceSelection {
    fn from(value: WorkflowTraceRuntimeSelection) -> Self {
        let ambiguous = value.is_ambiguous();
        Self {
            execution_id: value.execution_id,
            matched_execution_ids: value.matched_execution_ids,
            ambiguous,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRecoveryDebugState {
    pub in_progress: bool,
    pub attempt_count: u32,
    pub config: RecoveryConfig,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeDebugSnapshot {
    pub mode_info: ServerModeInfo,
    pub snapshot: pantograph_runtime_registry::RuntimeRegistrySnapshot,
    pub health_monitor_running: bool,
    pub last_health_check: Option<HealthCheckResult>,
    pub recovery: Option<RuntimeRecoveryDebugState>,
    pub workflow_runtime_diagnostics: Option<DiagnosticsRuntimeSnapshot>,
    pub workflow_scheduler_diagnostics: Option<DiagnosticsSchedulerSnapshot>,
    pub workflow_trace: Option<WorkflowTraceSnapshotResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_trace_selection: Option<RuntimeDebugTraceSelection>,
}

pub(crate) async fn runtime_registry_snapshot_response(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
) -> pantograph_runtime_registry::RuntimeRegistrySnapshot {
    synced_runtime_registry_snapshot(gateway.as_ref(), runtime_registry).await
}

pub(crate) async fn runtime_debug_snapshot_response(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    health_monitor: Option<&SharedHealthMonitor>,
    recovery_manager: Option<&SharedRecoveryManager>,
    workflow_diagnostics: Option<WorkflowDiagnosticsProjection>,
    workflow_trace: Option<WorkflowTraceSnapshotResponse>,
    workflow_trace_selection: Option<RuntimeDebugTraceSelection>,
) -> RuntimeDebugSnapshot {
    let mode_info = synced_server_mode_info(gateway, runtime_registry).await;
    let health_monitor_running = health_monitor
        .map(|monitor| monitor.is_running())
        .unwrap_or(false);
    let last_health_check = match health_monitor {
        Some(monitor) => monitor.last_result().await,
        None => None,
    };
    let recovery = match recovery_manager {
        Some(manager) => Some(RuntimeRecoveryDebugState {
            in_progress: manager.is_recovering(),
            attempt_count: manager.attempt_count(),
            config: manager.config().clone(),
            last_error: manager.last_error().await,
        }),
        None => None,
    };
    let workflow_runtime_diagnostics = workflow_diagnostics
        .as_ref()
        .map(|projection| projection.runtime.clone());
    let workflow_scheduler_diagnostics = workflow_diagnostics
        .as_ref()
        .map(|projection| projection.scheduler.clone());

    RuntimeDebugSnapshot {
        mode_info,
        snapshot: runtime_registry.snapshot(),
        health_monitor_running,
        last_health_check,
        recovery,
        workflow_runtime_diagnostics,
        workflow_scheduler_diagnostics,
        workflow_trace,
        workflow_trace_selection,
    }
}
