use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::HostRuntimeModeSnapshot;
use node_engine::{NodeEngineError, WorkflowExecutor};
use pantograph_runtime_identity::canonical_runtime_id;
use pantograph_runtime_registry::RuntimeRegistry;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionSummary, WorkflowSchedulerSnapshotDiagnostics,
    WorkflowSchedulerSnapshotResponse, WorkflowTraceRuntimeMetrics,
};

#[derive(Debug, Clone)]
pub struct RuntimeDiagnosticsProjection {
    pub active_runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub runtime_model_target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RuntimeEventProjection {
    pub active_runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionSchedulerSnapshot {
    pub workflow_id: Option<String>,
    pub workflow_run_id: Option<String>,
    pub session_id: String,
    pub captured_at_ms: u64,
    pub session: WorkflowExecutionSessionSummary,
    pub items: Vec<WorkflowExecutionSessionQueueItem>,
    pub diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionRuntimeSnapshot {
    pub workflow_id: String,
    pub workflow_run_id: Option<String>,
    pub captured_at_ms: u64,
    pub capabilities: Option<WorkflowCapabilitiesResponse>,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
    pub active_runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionDiagnosticsSnapshot {
    pub scheduler: WorkflowExecutionSchedulerSnapshot,
    pub runtime: WorkflowExecutionRuntimeSnapshot,
}

pub struct WorkflowExecutionDiagnosticsInput<'a> {
    pub runtime_registry: Option<&'a RuntimeRegistry>,
    pub scheduler_snapshot: &'a WorkflowSchedulerSnapshotResponse,
    pub captured_at_ms: u64,
    pub runtime_capabilities: Option<WorkflowCapabilitiesResponse>,
    pub runtime_error: Option<String>,
    pub trace_runtime_metrics_override: Option<WorkflowTraceRuntimeMetrics>,
    pub runtime_snapshot_override: Option<&'a inference::RuntimeLifecycleSnapshot>,
    pub gateway_snapshot: &'a inference::RuntimeLifecycleSnapshot,
    pub embedding_runtime_snapshot: Option<&'a inference::RuntimeLifecycleSnapshot>,
    pub gateway_mode_info: &'a HostRuntimeModeSnapshot,
    pub runtime_model_target_override: Option<&'a str>,
}

pub struct WorkflowExecutionDiagnosticsSyncInput<'a> {
    pub runtime_registry: Option<&'a RuntimeRegistry>,
    pub scheduler_snapshot: &'a WorkflowSchedulerSnapshotResponse,
    pub captured_at_ms: u64,
    pub runtime_capabilities: Option<WorkflowCapabilitiesResponse>,
    pub runtime_error: Option<String>,
    pub trace_runtime_metrics_override: Option<WorkflowTraceRuntimeMetrics>,
    pub runtime_snapshot_override: Option<&'a inference::RuntimeLifecycleSnapshot>,
    pub runtime_model_target_override: Option<&'a str>,
}

#[async_trait]
pub trait WorkflowExecutionDiagnosticsController:
    crate::runtime_registry::HostRuntimeRegistryController
{
    async fn active_runtime_lifecycle_snapshot(&self) -> inference::RuntimeLifecycleSnapshot;

    async fn embedding_runtime_lifecycle_snapshot(
        &self,
    ) -> Option<inference::RuntimeLifecycleSnapshot>;
}

pub fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

pub fn build_workflow_execution_diagnostics_snapshot(
    input: WorkflowExecutionDiagnosticsInput<'_>,
) -> WorkflowExecutionDiagnosticsSnapshot {
    let WorkflowExecutionDiagnosticsInput {
        runtime_registry,
        scheduler_snapshot,
        captured_at_ms,
        runtime_capabilities,
        runtime_error,
        trace_runtime_metrics_override,
        runtime_snapshot_override,
        gateway_snapshot,
        embedding_runtime_snapshot,
        gateway_mode_info,
        runtime_model_target_override,
    } = input;
    let workflow_id = scheduler_snapshot.workflow_id.clone();
    let runtime_workflow_id = workflow_id
        .clone()
        .unwrap_or_else(|| scheduler_snapshot.session.workflow_id.clone());
    let workflow_run_id = scheduler_snapshot.workflow_run_id.clone();
    let runtime_projection = build_runtime_event_projection_with_registry_override(
        runtime_registry,
        None,
        None,
        None,
        None,
        trace_runtime_metrics_override,
        runtime_snapshot_override,
        gateway_snapshot,
        embedding_runtime_snapshot,
        gateway_mode_info,
        runtime_model_target_override,
    );

    WorkflowExecutionDiagnosticsSnapshot {
        scheduler: WorkflowExecutionSchedulerSnapshot {
            workflow_id,
            workflow_run_id: workflow_run_id.clone(),
            session_id: scheduler_snapshot.session_id.clone(),
            captured_at_ms,
            session: scheduler_snapshot.session.clone(),
            items: scheduler_snapshot.items.clone(),
            diagnostics: scheduler_snapshot.diagnostics.clone(),
        },
        runtime: WorkflowExecutionRuntimeSnapshot {
            workflow_id: runtime_workflow_id,
            workflow_run_id,
            captured_at_ms,
            capabilities: runtime_capabilities,
            trace_runtime_metrics: runtime_projection.trace_runtime_metrics,
            active_model_target: runtime_projection.active_model_target,
            embedding_model_target: runtime_projection.embedding_model_target,
            active_runtime_snapshot: runtime_projection.active_runtime_snapshot,
            embedding_runtime_snapshot: runtime_projection.embedding_runtime_snapshot,
            error: runtime_error,
        },
    }
}

pub async fn build_workflow_execution_diagnostics_snapshot_with_registry_sync<C>(
    controller: &C,
    input: WorkflowExecutionDiagnosticsSyncInput<'_>,
) -> WorkflowExecutionDiagnosticsSnapshot
where
    C: WorkflowExecutionDiagnosticsController + Sync,
{
    let WorkflowExecutionDiagnosticsSyncInput {
        runtime_registry,
        scheduler_snapshot,
        captured_at_ms,
        runtime_capabilities,
        runtime_error,
        trace_runtime_metrics_override,
        runtime_snapshot_override,
        runtime_model_target_override,
    } = input;
    if let Some(registry) = runtime_registry {
        crate::runtime_registry::sync_runtime_registry(controller, registry).await;
    }

    let gateway_snapshot = controller.active_runtime_lifecycle_snapshot().await;
    let embedding_runtime_snapshot = controller.embedding_runtime_lifecycle_snapshot().await;
    let gateway_mode_info = controller.mode_info_snapshot().await;

    build_workflow_execution_diagnostics_snapshot(WorkflowExecutionDiagnosticsInput {
        runtime_registry,
        scheduler_snapshot,
        captured_at_ms,
        runtime_capabilities,
        runtime_error,
        trace_runtime_metrics_override,
        runtime_snapshot_override,
        gateway_snapshot: &gateway_snapshot,
        embedding_runtime_snapshot: embedding_runtime_snapshot.as_ref(),
        gateway_mode_info: &gateway_mode_info,
        runtime_model_target_override,
    })
}

pub async fn sync_embedding_emit_metadata_flags(
    executor: &mut WorkflowExecutor,
) -> Result<(), NodeEngineError> {
    let snapshot = executor.get_graph_snapshot().await;
    let mut counts = HashMap::<String, u32>::new();
    for edge in &snapshot.edges {
        let key = format!("{}:{}", edge.source, edge.source_handle);
        counts
            .entry(key)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    for node in &snapshot.nodes {
        if node.node_type != "embedding" {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        let mut data = node.data.clone();
        match data {
            serde_json::Value::Object(ref mut map) => {
                map.insert(
                    "emit_metadata".to_string(),
                    serde_json::json!(emit_metadata),
                );
            }
            _ => {
                data = serde_json::json!({ "emit_metadata": emit_metadata });
            }
        }
        executor.update_node_data(&node.id, data).await?;
    }

    Ok(())
}

pub fn trace_runtime_metrics(
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_target: Option<&str>,
) -> WorkflowTraceRuntimeMetrics {
    let observed_runtime_ids = observed_runtime_ids(snapshot, &[]);
    let runtime_id = observed_runtime_ids.first().cloned();
    WorkflowTraceRuntimeMetrics {
        runtime_id: runtime_id.clone(),
        observed_runtime_ids,
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        model_target: model_target.map(ToOwned::to_owned),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.normalized_lifecycle_decision_reason(),
    }
}

pub fn normalized_runtime_lifecycle_snapshot(
    snapshot: &inference::RuntimeLifecycleSnapshot,
) -> inference::RuntimeLifecycleSnapshot {
    inference::RuntimeLifecycleSnapshot {
        runtime_id: snapshot
            .runtime_id
            .as_deref()
            .map(canonical_runtime_id)
            .filter(|runtime_id| !runtime_id.is_empty()),
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.normalized_lifecycle_decision_reason(),
        active: snapshot.active,
        last_error: snapshot.last_error.clone(),
    }
}

pub fn trace_runtime_metrics_with_observed_runtime_ids(
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_target: Option<&str>,
    additional_observed_runtime_ids: &[String],
) -> WorkflowTraceRuntimeMetrics {
    let observed_runtime_ids = observed_runtime_ids(snapshot, additional_observed_runtime_ids);
    let runtime_id = observed_runtime_ids.first().cloned();
    WorkflowTraceRuntimeMetrics {
        runtime_id,
        observed_runtime_ids,
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        model_target: model_target.map(ToOwned::to_owned),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.normalized_lifecycle_decision_reason(),
    }
}

fn observed_runtime_ids(
    snapshot: &inference::RuntimeLifecycleSnapshot,
    additional_observed_runtime_ids: &[String],
) -> Vec<String> {
    let mut observed_runtime_ids = snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty())
        .into_iter()
        .collect::<Vec<_>>();
    for runtime_id in additional_observed_runtime_ids {
        let runtime_id = canonical_runtime_id(runtime_id);
        if runtime_id.is_empty() || observed_runtime_ids.contains(&runtime_id) {
            continue;
        }
        observed_runtime_ids.push(runtime_id);
    }
    observed_runtime_ids
}

pub fn resolve_runtime_model_target(
    mode_info: &HostRuntimeModeSnapshot,
    snapshot: &inference::RuntimeLifecycleSnapshot,
) -> Option<String> {
    if snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .as_deref()
        == Some("llama.cpp.embedding")
    {
        return mode_info.embedding_model_target.clone();
    }
    mode_info.active_model_target.clone()
}

pub fn capability_runtime_lifecycle_snapshot(
    capabilities: Option<&WorkflowCapabilitiesResponse>,
) -> Option<inference::RuntimeLifecycleSnapshot> {
    crate::runtime_capabilities::capability_runtime_lifecycle_snapshot(capabilities)
}

pub fn build_runtime_diagnostics_projection(
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    gateway_mode_info: &HostRuntimeModeSnapshot,
    runtime_model_target_override: Option<&str>,
) -> RuntimeDiagnosticsProjection {
    let projection = build_runtime_event_projection(
        None,
        None,
        None,
        None,
        None,
        runtime_snapshot_override,
        gateway_snapshot,
        None,
        gateway_mode_info,
        runtime_model_target_override,
    );

    RuntimeDiagnosticsProjection {
        active_runtime_snapshot: projection.active_runtime_snapshot,
        trace_runtime_metrics: projection.trace_runtime_metrics,
        runtime_model_target: projection.active_model_target,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_runtime_event_projection_with_registry_override(
    runtime_registry: Option<&RuntimeRegistry>,
    stored_active_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_active_model_target: Option<&str>,
    stored_embedding_model_target: Option<&str>,
    stored_trace_runtime_metrics: Option<WorkflowTraceRuntimeMetrics>,
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_mode_info: &HostRuntimeModeSnapshot,
    runtime_model_target_override: Option<&str>,
) -> RuntimeEventProjection {
    let projection = build_runtime_event_projection(
        stored_active_runtime_snapshot,
        stored_embedding_runtime_snapshot,
        stored_active_model_target,
        stored_embedding_model_target,
        stored_trace_runtime_metrics,
        runtime_snapshot_override,
        gateway_snapshot,
        embedding_runtime_snapshot,
        gateway_mode_info,
        runtime_model_target_override,
    );
    reconcile_runtime_projection_registry_override(
        runtime_registry,
        runtime_snapshot_override,
        projection.active_model_target.as_deref(),
    );
    projection
}

#[allow(clippy::too_many_arguments)]
pub fn build_runtime_event_projection_with_registry_reconciliation(
    runtime_registry: Option<&RuntimeRegistry>,
    stored_active_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_active_model_target: Option<&str>,
    stored_embedding_model_target: Option<&str>,
    stored_trace_runtime_metrics: Option<WorkflowTraceRuntimeMetrics>,
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_mode_info: &HostRuntimeModeSnapshot,
    runtime_model_target_override: Option<&str>,
) -> RuntimeEventProjection {
    reconcile_runtime_registry_stored_projection_overrides(
        runtime_registry,
        stored_active_runtime_snapshot,
        stored_embedding_runtime_snapshot,
        stored_active_model_target,
        stored_embedding_model_target,
        gateway_mode_info,
    );
    build_runtime_event_projection_with_registry_override(
        runtime_registry,
        stored_active_runtime_snapshot,
        stored_embedding_runtime_snapshot,
        stored_active_model_target,
        stored_embedding_model_target,
        stored_trace_runtime_metrics,
        runtime_snapshot_override,
        gateway_snapshot,
        embedding_runtime_snapshot,
        gateway_mode_info,
        runtime_model_target_override,
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn build_runtime_event_projection_with_registry_sync<C>(
    controller: &C,
    runtime_registry: &RuntimeRegistry,
    stored_active_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_active_model_target: Option<&str>,
    stored_embedding_model_target: Option<&str>,
    stored_trace_runtime_metrics: Option<WorkflowTraceRuntimeMetrics>,
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_mode_info: &HostRuntimeModeSnapshot,
    runtime_model_target_override: Option<&str>,
) -> RuntimeEventProjection
where
    C: crate::runtime_registry::HostRuntimeRegistryController + Sync,
{
    crate::runtime_registry::sync_runtime_registry(controller, runtime_registry).await;
    build_runtime_event_projection_with_registry_reconciliation(
        Some(runtime_registry),
        stored_active_runtime_snapshot,
        stored_embedding_runtime_snapshot,
        stored_active_model_target,
        stored_embedding_model_target,
        stored_trace_runtime_metrics,
        runtime_snapshot_override,
        gateway_snapshot,
        embedding_runtime_snapshot,
        gateway_mode_info,
        runtime_model_target_override,
    )
}

pub fn reconcile_runtime_registry_stored_projection_overrides(
    runtime_registry: Option<&RuntimeRegistry>,
    stored_active_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_active_model_target: Option<&str>,
    stored_embedding_model_target: Option<&str>,
    gateway_mode_info: &HostRuntimeModeSnapshot,
) {
    let Some(runtime_registry) = runtime_registry else {
        return;
    };

    crate::runtime_registry::reconcile_runtime_registry_stored_projection_overrides(
        runtime_registry,
        stored_active_runtime_snapshot,
        stored_embedding_runtime_snapshot,
        stored_active_model_target,
        stored_embedding_model_target,
        gateway_mode_info,
    );
}

fn reconcile_runtime_projection_registry_override(
    runtime_registry: Option<&RuntimeRegistry>,
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    runtime_model_target: Option<&str>,
) {
    let (Some(runtime_registry), Some(runtime_snapshot_override)) =
        (runtime_registry, runtime_snapshot_override)
    else {
        return;
    };

    crate::runtime_registry::reconcile_runtime_registry_snapshot_override(
        runtime_registry,
        runtime_snapshot_override,
        runtime_model_target,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn build_runtime_event_projection(
    stored_active_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_active_model_target: Option<&str>,
    stored_embedding_model_target: Option<&str>,
    stored_trace_runtime_metrics: Option<WorkflowTraceRuntimeMetrics>,
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_mode_info: &HostRuntimeModeSnapshot,
    runtime_model_target_override: Option<&str>,
) -> RuntimeEventProjection {
    let active_runtime_snapshot = runtime_snapshot_override
        .cloned()
        .or_else(|| stored_active_runtime_snapshot.cloned())
        .unwrap_or_else(|| gateway_snapshot.clone());
    let embedding_runtime_snapshot = stored_embedding_runtime_snapshot
        .cloned()
        .or_else(|| embedding_runtime_snapshot.cloned());
    let active_model_target = runtime_model_target_override
        .map(ToOwned::to_owned)
        .or_else(|| stored_active_model_target.map(ToOwned::to_owned))
        .or_else(|| resolve_runtime_model_target(gateway_mode_info, &active_runtime_snapshot));
    let embedding_model_target = stored_embedding_model_target
        .map(ToOwned::to_owned)
        .or_else(|| gateway_mode_info.embedding_model_target.clone());
    let trace_runtime_metrics = stored_trace_runtime_metrics.unwrap_or_else(|| {
        trace_runtime_metrics(&active_runtime_snapshot, active_model_target.as_deref())
    });

    RuntimeEventProjection {
        active_runtime_snapshot,
        embedding_runtime_snapshot,
        trace_runtime_metrics,
        active_model_target,
        embedding_model_target,
    }
}

#[cfg(test)]
#[path = "workflow_runtime_tests.rs"]
mod tests;
