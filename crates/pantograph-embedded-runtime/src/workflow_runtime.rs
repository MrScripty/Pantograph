use std::collections::HashMap;

use node_engine::{NodeEngineError, WorkflowExecutor};
use pantograph_runtime_identity::canonical_runtime_id;
use pantograph_workflow_service::WorkflowTraceRuntimeMetrics;

#[derive(Debug, Clone)]
pub struct RuntimeDiagnosticsProjection {
    pub active_runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub runtime_model_target: Option<String>,
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
    let runtime_id = snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty());
    WorkflowTraceRuntimeMetrics {
        runtime_id: runtime_id.clone(),
        observed_runtime_ids: runtime_id.into_iter().collect(),
        runtime_instance_id: snapshot.runtime_instance_id.clone(),
        model_target: model_target.map(ToOwned::to_owned),
        warmup_started_at_ms: snapshot.warmup_started_at_ms,
        warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
        warmup_duration_ms: snapshot.warmup_duration_ms,
        runtime_reused: snapshot.runtime_reused,
        lifecycle_decision_reason: snapshot.normalized_lifecycle_decision_reason(),
    }
}

pub fn resolve_runtime_model_target(
    mode_info: &inference::ServerModeInfo,
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

pub fn build_runtime_diagnostics_projection(
    runtime_snapshot_override: Option<&inference::RuntimeLifecycleSnapshot>,
    gateway_snapshot: &inference::RuntimeLifecycleSnapshot,
    gateway_mode_info: &inference::ServerModeInfo,
    runtime_model_target_override: Option<&str>,
) -> RuntimeDiagnosticsProjection {
    let active_runtime_snapshot = runtime_snapshot_override
        .cloned()
        .unwrap_or_else(|| gateway_snapshot.clone());
    let runtime_model_target = runtime_snapshot_override
        .and_then(|snapshot| {
            runtime_model_target_override
                .map(ToOwned::to_owned)
                .or_else(|| resolve_runtime_model_target(gateway_mode_info, snapshot))
        })
        .or_else(|| resolve_runtime_model_target(gateway_mode_info, gateway_snapshot));
    let trace_runtime_metrics = trace_runtime_metrics(
        runtime_snapshot_override.unwrap_or(gateway_snapshot),
        runtime_model_target.as_deref(),
    );

    RuntimeDiagnosticsProjection {
        active_runtime_snapshot,
        trace_runtime_metrics,
        runtime_model_target,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_runtime_diagnostics_projection, resolve_runtime_model_target, trace_runtime_metrics,
    };

    #[test]
    fn trace_runtime_metrics_keeps_canonical_backend_lifecycle_reason() {
        let metrics = trace_runtime_metrics(
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("pytorch".to_string()),
                runtime_instance_id: Some("pytorch-1".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
                active: true,
                last_error: None,
            },
            Some("/models/demo"),
        );

        assert_eq!(
            metrics.lifecycle_decision_reason.as_deref(),
            Some("runtime_reused")
        );
        assert_eq!(metrics.model_target.as_deref(), Some("/models/demo"));
    }

    #[test]
    fn trace_runtime_metrics_normalizes_known_runtime_aliases() {
        let metrics = trace_runtime_metrics(
            &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-cpp-1".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            },
            Some("/models/main.gguf"),
        );

        assert_eq!(metrics.runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(metrics.observed_runtime_ids, vec!["llama_cpp".to_string()]);
    }

    #[test]
    fn resolve_runtime_model_target_prefers_embedding_target_for_embedding_alias() {
        let mode_info = inference::ServerModeInfo {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            mode: "sidecar_inference".to_string(),
            ready: true,
            url: None,
            model_path: None,
            is_embedding_mode: false,
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: None,
        };
        let snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama_cpp_embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        };

        assert_eq!(
            resolve_runtime_model_target(&mode_info, &snapshot).as_deref(),
            Some("/models/embed.gguf")
        );
    }

    #[test]
    fn build_runtime_diagnostics_projection_prefers_execution_snapshot_override() {
        let execution_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
            warmup_started_at_ms: Some(100),
            warmup_completed_at_ms: Some(110),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        };
        let restored_gateway_snapshot = inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-restore-9".to_string()),
            warmup_started_at_ms: Some(200),
            warmup_completed_at_ms: Some(240),
            warmup_duration_ms: Some(40),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        };
        let mode_info = inference::ServerModeInfo {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            mode: "sidecar_inference".to_string(),
            ready: true,
            url: None,
            model_path: None,
            is_embedding_mode: false,
            active_model_target: Some("/models/restore.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: None,
        };

        let projection = build_runtime_diagnostics_projection(
            Some(&execution_snapshot),
            &restored_gateway_snapshot,
            &mode_info,
            Some("/models/embed.gguf"),
        );

        assert_eq!(
            projection
                .active_runtime_snapshot
                .runtime_instance_id
                .as_deref(),
            Some("llama-cpp-embedding-2")
        );
        assert_eq!(
            projection.runtime_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        assert_eq!(
            projection.trace_runtime_metrics.runtime_id.as_deref(),
            Some("llama.cpp.embedding")
        );
        assert_eq!(
            projection.trace_runtime_metrics.observed_runtime_ids,
            vec!["llama.cpp.embedding".to_string()]
        );
        assert_eq!(
            projection
                .trace_runtime_metrics
                .lifecycle_decision_reason
                .as_deref(),
            Some("runtime_reused")
        );
    }
}
